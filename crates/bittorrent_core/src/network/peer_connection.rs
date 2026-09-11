use core::panic;
use std::{
    net::SocketAddr,
    ops::{Add, Not},
    sync::Arc,
    time::{Duration, SystemTime},
};

use log::info;
use tokio::{
    io::AsyncReadExt,
    net::{TcpStream, tcp::OwnedReadHalf},
    sync::{
        Mutex, Notify,
        mpsc::{self, Receiver, Sender},
    },
    task::JoinHandle,
    time::interval,
};
use winnow::{
    Parser,
    binary::{be_u8, be_u32},
    token::rest,
};

use crate::{
    bitfield::Bitfield,
    network::{
        BitTorrentStream, BitTorrentStreamReader, BitTorrentStreamWriter, Queue, message::Message,
    },
    pieces::{Block, BlockRequest},
};

pub enum BubbledMessage {
    Request(BlockRequest),
    Piece(Block),
    Have(usize),
}
#[derive(Debug)]
pub struct Peer {
    pub(crate) id: Option<Vec<u8>>,
    pub(crate) socket: SocketAddr,
}
#[derive(Debug)]
pub struct InactivePeerConnection {
    id: [u8; 20],
    remote_socket: SocketAddr,
    stream: BitTorrentStream,
}

pub struct PeerConnectionIo {
    pub bubble_recv: Receiver<Message>,
    pub shutdown: Arc<Notify>,
}

#[derive(Debug)]
pub struct PeerConnection {
    id: [u8; 20],
    choke_interest_data: u8,
    remote_socket: SocketAddr,
    stream: BitTorrentStreamWriter,

    reader_task: JoinHandle<()>,
    message_handler_task: Option<JoinHandle<()>>,

    bubble_sender: Sender<Message>,

    close_at: SystemTime,

    on_shutdown: Arc<Notify>,
}

impl InactivePeerConnection {
    pub(crate) fn new(
        id: [u8; 20],
        remote_socket: SocketAddr,
        stream: impl Into<BitTorrentStream>,
    ) -> Self {
        Self {
            id,
            remote_socket,
            stream: stream.into(),
        }
    }

    pub(crate) async fn activate(self) -> (Arc<Mutex<PeerConnection>>, PeerConnectionIo) {
        let InactivePeerConnection {
            id,
            remote_socket,
            stream,
        } = self;

        match stream {
            BitTorrentStream::Tcp(stream) => {
                PeerConnection::new_tcp(id, remote_socket, stream).await
            }
            BitTorrentStream::Utp(_stream) => unimplemented!(),
        }
    }
    pub fn id(&self) -> &[u8; 20] {
        &self.id
    }
}

impl PeerConnection {
    async fn new_tcp(
        id: [u8; 20],
        remote_socket: SocketAddr,
        stream: TcpStream,
    ) -> (Arc<Mutex<Self>>, PeerConnectionIo) {
        let (reader, writer) = stream.into_split();

        let keepalive_notify = Arc::new(Notify::const_new());
        let shutdown_notify = Arc::new(Notify::const_new());

        let (bubble_send, bubble_recv) = mpsc::channel(100);

        let (msg_send, mut msg_recv) = tokio::sync::mpsc::channel(100);

        let reader_task = start_reader(
            reader.into(),
            msg_send,
            Arc::clone(&keepalive_notify),
            Arc::clone(&shutdown_notify),
        );

        let on_shutdown = Arc::new(Notify::const_new());

        let connection = Arc::new(Mutex::new(Self {
            id,
            choke_interest_data: 0,
            remote_socket,
            stream: writer.into(),
            close_at: SystemTime::now().add(Duration::from_mins(2)),
            reader_task: reader_task,
            bubble_sender: bubble_send,
            message_handler_task: None,

            on_shutdown: Arc::clone(&on_shutdown),
        }));
        // disconnect task
        {
            let shutdown_notify = Arc::clone(&shutdown_notify);
            let connection = Arc::clone(&connection);
            tokio::spawn(async move {
                shutdown_notify.notified_owned().await;
                connection.lock().await.shutdown().await;
            });
        }
        // keepalive task
        {
            let keepalive_notify = Arc::clone(&keepalive_notify);
            let connection = Arc::clone(&connection);
            tokio::spawn(async move {
                loop {
                    keepalive_notify.notified().await;
                    connection.lock().await.keepalive();
                }
            });
        }
        // periodically check if the timer has elapsed (task)
        {
            let connection = Arc::clone(&connection);

            tokio::spawn(async move {
                let mut interval = interval(Duration::from_secs(1));
                loop {
                    interval.tick().await;
                    let now = SystemTime::now();
                    let mut lock = connection.lock().await;
                    if now < connection.lock().await.close_at {
                        lock.shutdown().await;
                    }
                }
            });
        }
        // message handler task
        let message_handler_task = {
            let connection = Arc::clone(&connection);
            Some(tokio::spawn(async move {
                while let Some(message) = msg_recv.recv().await {
                    connection.lock().await.handle_message(message);
                }
            }))
        };
        connection.lock().await.message_handler_task = message_handler_task;

        (
            connection,
            PeerConnectionIo {
                bubble_recv,
                shutdown: on_shutdown,
            },
        )
    }
    pub async fn shutdown(&mut self) {
        self.reader_task.abort();
        self.on_shutdown.notify_waiters();
    }
    pub fn keepalive(&mut self) {
        self.close_at = SystemTime::now().add(Duration::from_mins(2));
    }
    pub fn handle_message(&mut self, message: Message) {
        info!("Received message: {:?}", message);
        match message {
            Message::KeepAlive => self.keepalive(),
            Message::Choke => self.set_is_choking(true),
            Message::Unchoke => self.set_is_choking(false),
            Message::Interested => self.set_is_interested(true),
            Message::NotInterested => self.set_is_interested(false),
            msg => {
                let _ = self.bubble_sender.send(msg);
            }
        }
        panic!();
    }

    pub async fn send_handshake(&mut self, info_hash: &[u8; 20], peer_id: &[u8; 20]) {
        self.stream.send_handshake(info_hash, peer_id).await
    }
    pub fn id(&self) -> &[u8; 20] {
        &self.id
    }
    pub fn is_choking(&self) -> bool {
        self.choke_interest_data & 0b1 != 0
    }
    pub fn is_interested(&self) -> bool {
        self.choke_interest_data & 0b10 != 0
    }
    pub fn am_choking(&self) -> bool {
        self.choke_interest_data & 0b100 != 0
    }
    pub fn am_interested(&self) -> bool {
        self.choke_interest_data & 0b1000 != 0
    }

    pub fn set_is_choking(&mut self, val: bool) {
        if val {
            self.choke_interest_data |= 1;
        } else {
            self.choke_interest_data &= !1;
        }
    }
    pub fn set_is_interested(&mut self, val: bool) {
        if val {
            self.choke_interest_data |= 1;
        } else {
            self.choke_interest_data &= !1;
        }
    }
}

fn start_reader(
    reader: BitTorrentStreamReader,
    msg_send: Sender<Message>,
    keepalive_notify: Arc<Notify>,
    shutdown_notify: Arc<Notify>,
) -> JoinHandle<()> {
    match reader {
        BitTorrentStreamReader::Tcp(reader) => tokio::spawn(start_tcp_reader(
            reader,
            msg_send,
            shutdown_notify,
            keepalive_notify,
        )),
        BitTorrentStreamReader::Utp(_) => unimplemented!(),
    }
}

async fn start_tcp_reader(
    mut reader: OwnedReadHalf,
    msg_send: Sender<Message>,
    shutdown_notify: Arc<Notify>,
    keepalive_notify: Arc<Notify>,
) {
    // 32 KiB should be enough for anything
    let mut buffer = vec![0u8; 32 * 1024];
    loop {
        // read length
        let Ok(length) = reader.read_u32().await else {
            shutdown_notify.notify_waiters();
            break;
        };
        keepalive_notify.notify_waiters();
        if length == 0 {
            continue;
        }
        reader
            .read_exact(&mut buffer[0..length as usize] as _)
            .await
            .unwrap();

        let Ok(message) = parse_message(&buffer[0..length as usize]) else {
            shutdown_notify.notify_waiters();
            break;
        };

        let Ok(_) = msg_send.send(message).await else {
            shutdown_notify.notify_waiters();
            break;
        };
    }
}
fn parse_message(mut message: &[u8]) -> winnow::Result<Message> {
    let index = be_u8.parse_next(&mut message)?;
    match index {
        0 => return Ok(Message::Choke),
        1 => return Ok(Message::Unchoke),
        2 => return Ok(Message::Interested),
        3 => return Ok(Message::NotInterested),
        4 => return Ok(Message::Have(be_u32.parse_next(&mut message)? as usize)),
        5 => return Ok(Message::Bitfield(Bitfield::new(message))),
        6 => {
            let (index, begin, length) = (be_u32, be_u32, be_u32).parse_next(&mut message)?;
            return Ok(Message::Request {
                index: index as usize,
                begin: begin as usize,
                length: length as usize,
            });
        }
        7 => {
            let (index, begin, block) = (be_u32, be_u32, rest).parse_next(&mut message)?;
            return Ok(Message::Piece {
                index: index as usize,
                begin: begin as usize,
                block: block.to_vec(),
            });
        }
        8 => {
            let (index, begin, length) = (be_u32, be_u32, be_u32).parse_next(&mut message)?;
            return Ok(Message::Cancel {
                index: index as usize,
                begin: begin as usize,
                length: length as usize,
            });
        }
        _ => unimplemented!(),
    }
}
