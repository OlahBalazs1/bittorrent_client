#![feature(sync_nonpoison)]
use core::panic;
use std::{
    collections::HashMap,
    net::SocketAddr,
    ops::Add,
    sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
    },
    time::{Duration, SystemTime},
};

use std::sync::nonpoison::Mutex;

use log::{info, warn};
use tokio::{
    io::AsyncReadExt,
    net::{TcpStream, tcp::OwnedReadHalf},
    sync::{
        Mutex as TokioMutex, Notify,
        mpsc::{self, Receiver, Sender},
        oneshot,
    },
    task::{AbortHandle, JoinHandle, JoinSet},
    time::interval,
};
use winnow::{
    Parser,
    binary::{be_u8, be_u32},
    token::rest,
};

use crate::{
    bitfield::Bitfield,
    network::{BitTorrentStream, BitTorrentStreamReader, BitTorrentStreamWriter, message::Message},
    pieces::{Block, BlockRequest},
};

pub(crate) enum BubbledMessage {
    Request(BlockRequest),
    Piece(Block),
    Have(usize),
    Bitfield(Bitfield),
}
#[derive(Debug)]
pub struct Peer {
    pub(crate) id: Option<[u8; 20]>,
    pub(crate) socket: SocketAddr,
}
#[derive(Debug)]
pub struct InactivePeerConnection {
    id: [u8; 20],
    remote_socket: SocketAddr,
    stream: BitTorrentStream,
}

pub(crate) struct PeerConnectionIo {
    pub bubble_recv: Receiver<BubbledMessage>,
    pub on_shutdown: Arc<Notify>,
}

#[derive(Debug)]
pub struct PeerConnection {
    id: [u8; 20],
    choke_interest_data: AtomicU8,
    remote_socket: SocketAddr,

    stream: TokioMutex<BitTorrentStreamWriter>,
    tasks: Mutex<JoinSet<()>>,

    // (piece, begin, length): JoinHandle
    standing_requests: Mutex<HashMap<(usize, usize, usize), AbortHandle>>,

    bubble_sender: Sender<BubbledMessage>,

    on_shutdown: Arc<Notify>,

    kept_alive: Arc<Notify>,
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

    pub(crate) async fn activate(self) -> (Arc<PeerConnection>, PeerConnectionIo) {
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
    ) -> (Arc<Self>, PeerConnectionIo) {
        let (reader, writer) = stream.into_split();

        let keepalive_notify = Arc::new(Notify::const_new());
        let shutdown_notify = Arc::new(Notify::const_new());

        let (bubble_send, bubble_recv) = mpsc::channel::<BubbledMessage>(128);

        let (msg_send, mut msg_recv) = tokio::sync::mpsc::channel(128);

        let tasks = Mutex::new(JoinSet::new());

        tasks.lock().spawn(start_reader(
            reader.into(),
            msg_send,
            Arc::clone(&keepalive_notify),
            Arc::clone(&shutdown_notify),
        ));

        let on_shutdown = Arc::new(Notify::const_new());

        let conn = Arc::new(Self {
            id,
            choke_interest_data: AtomicU8::new(0),
            remote_socket,
            stream: TokioMutex::new(BitTorrentStreamWriter::from(writer)),

            kept_alive: Arc::new(Notify::const_new()),

            tasks,
            standing_requests: Mutex::new(HashMap::new()),
            bubble_sender: bubble_send,

            on_shutdown: Arc::clone(&on_shutdown),
        });
        // disconnect task
        {
            let shutdown_notify = Arc::clone(&shutdown_notify);
            let connection = Arc::clone(&conn);
            conn.tasks.lock().spawn(async move {
                shutdown_notify.notified_owned().await;
                connection.shutdown().await;
            });
        }
        // keepalive task
        {
            let keepalive_notify = Arc::clone(&keepalive_notify);
            let connection = Arc::clone(&conn);
            conn.tasks.lock().spawn(async move {
                loop {
                    keepalive_notify.notified().await;
                    connection.keepalive();
                }
            });
        }
        // periodically check if the timer has elapsed (task)
        {
            let connection = Arc::clone(&conn);

            conn.tasks.lock().spawn(async move {
                loop {
                    tokio::select! {
                        _ = tokio::time::sleep(Duration::from_mins(2)) => {
                            connection.shutdown().await
                        }
                        // restart the wait if it is kept alive
                        _ = connection.kept_alive.notified() => {}
                    }
                }
            });
        }
        // message handler task
        {
            let connection = Arc::clone(&conn);
            conn.tasks.lock().spawn(async move {
                while let Some(message) = msg_recv.recv().await {
                    Arc::clone(&connection).handle_message(message).await;
                }
            })
        };

        (
            conn,
            PeerConnectionIo {
                bubble_recv,
                on_shutdown,
            },
        )
    }
    pub async fn shutdown(&self) {
        self.tasks.lock().abort_all();
        self.on_shutdown.notify_waiters();
    }
    pub fn keepalive(&self) {
        self.kept_alive.notify_waiters();
    }
    pub async fn handle_message(self: Arc<Self>, message: Message) {
        info!("Received message: {:?}", message);
        match message {
            Message::KeepAlive => self.keepalive(),
            Message::Choke => self.set_is_choking(true),
            Message::Unchoke => self.set_is_choking(false),
            Message::Interested => self.set_is_interested(true),
            Message::NotInterested => self.set_is_interested(false),

            Message::Request {
                index,
                begin,
                length,
            } => {
                Arc::clone(&self)
                    .bubble(BubbledMessage::Request(
                        self.create_request(index, begin, length),
                    ))
                    .await
            }
            Message::Have(have) => self.bubble(BubbledMessage::Have(have)).await,
            Message::Bitfield(bitfield) => self.bubble(BubbledMessage::Bitfield(bitfield)).await,
            Message::Piece(block) => self.bubble(BubbledMessage::Piece(block)).await,
            Message::Cancel {
                piece,
                begin,
                length,
            } => self.cancel_request(piece, begin, length),
        }
        panic!();
    }

    pub async fn send_handshake(&self, info_hash: &[u8; 20], peer_id: &[u8; 20]) {
        self.stream
            .lock()
            .await
            .send_handshake(info_hash, peer_id)
            .await;
    }
    pub fn id(&self) -> &[u8; 20] {
        &self.id
    }
    pub fn is_choking(&self) -> bool {
        self.choke_interest_data.load(Ordering::SeqCst) & 0b1 != 0
    }
    pub fn is_interested(&self) -> bool {
        self.choke_interest_data.load(Ordering::SeqCst) & 0b10 != 0
    }
    pub fn am_choking(&self) -> bool {
        self.choke_interest_data.load(Ordering::SeqCst) & 0b100 != 0
    }
    pub fn am_interested(&self) -> bool {
        self.choke_interest_data.load(Ordering::SeqCst) & 0b1000 != 0
    }

    pub fn set_is_choking(&self, val: bool) {
        if val {
            self.choke_interest_data.fetch_or(1, Ordering::SeqCst);
        } else {
            self.choke_interest_data.fetch_and(!1, Ordering::SeqCst);
        }
    }
    pub fn set_is_interested(&self, val: bool) {
        if val {
            self.choke_interest_data.fetch_or(0b10, Ordering::SeqCst);
        } else {
            self.choke_interest_data.fetch_and(!0b10, Ordering::SeqCst);
        }
    }
    pub fn set_am_choking(&self, val: bool) {
        if val {
            self.choke_interest_data.fetch_or(0b100, Ordering::SeqCst);
        } else {
            self.choke_interest_data.fetch_and(!0b100, Ordering::SeqCst);
        }
    }
    pub fn set_am_interested(&self, val: bool) {
        if val {
            self.choke_interest_data.fetch_or(0b1000, Ordering::SeqCst);
        } else {
            self.choke_interest_data
                .fetch_and(!0b1000, Ordering::SeqCst);
        }
    }

    fn cancel_request(&self, piece: usize, begin: usize, length: usize) {
        if let None = self
            .standing_requests
            .lock()
            .remove(&(piece, begin, length))
        {
            info!(
                "Peer with ID {} tried to cancel an invalid request.",
                String::from_utf8_lossy(self.id())
            );
        };
    }
    fn create_request(self: Arc<Self>, piece: usize, begin: usize, length: usize) -> BlockRequest {
        let (block_send, block_recv) = oneshot::channel();
        let request = BlockRequest::new(piece, begin, length, block_send);

        // dropping the receiver by aborting this task implicitly invalidates the sender, thus canceling the BlockRequest
        let handle = {
            let delegate = Arc::clone(&self);
            self.tasks.lock().spawn(async move {
                let Ok(block) = block_recv.await else {
                    return;
                };
                delegate.stream.lock().await.send_block(block).await;
                delegate
                    .standing_requests
                    .lock()
                    .remove(&(piece, begin, length));
            })
        };
        self.standing_requests
            .lock()
            .insert((piece, begin, length), handle);

        request
    }
    async fn bubble(&self, message: BubbledMessage) {
        let Ok(_) = self.bubble_sender.send(message).await else {
            self.shutdown().await;
            warn!(
                "Connection to {} could not bubble message, cutting connection!",
                String::from_utf8_lossy(self.id())
            );
            return;
        };
    }
}

async fn start_reader(
    reader: BitTorrentStreamReader,
    msg_send: Sender<Message>,
    keepalive_notify: Arc<Notify>,
    shutdown_notify: Arc<Notify>,
) {
    match reader {
        BitTorrentStreamReader::Tcp(reader) => {
            return start_tcp_reader(reader, msg_send, shutdown_notify, keepalive_notify).await;
        }
        BitTorrentStreamReader::Utp(_) => unimplemented!(),
    }
}

// TODO: drop connection if message exceeds a certain size
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
            return Ok(Message::Piece(Block {
                piece: index as usize,
                begin: begin as usize,
                data: block.into(),
            }));
        }
        8 => {
            let (index, begin, length) = (be_u32, be_u32, be_u32).parse_next(&mut message)?;
            return Ok(Message::Cancel {
                piece: index as usize,
                begin: begin as usize,
                length: length as usize,
            });
        }
        _ => unimplemented!(),
    }
}
