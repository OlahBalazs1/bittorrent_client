use std::{
    net::SocketAddr,
    ops::Add,
    sync::Arc,
    time::{Duration, SystemTime},
};

use tokio::{net::TcpStream, sync::Mutex, task::JoinHandle};

use crate::network::{BitTorrentStream, BitTorrentStreamReader, BitTorrentStreamWriter, Queue, message::Message};
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

#[derive(Debug)]
pub struct PeerConnection {
    id: [u8; 20],
    choke_interest_data: u8,
    remote_socket: SocketAddr,
    stream: BitTorrentStreamWriter,

    reader_task: Option<JoinHandle<()>>,

    close_at: Arc<Mutex<SystemTime>>,
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

    pub(crate) fn activate(self, incoming_buffer: Arc<Mutex<Queue<Message>>>) -> PeerConnection {
        let InactivePeerConnection {
            id,
            remote_socket,
            stream,
        } = self;

        match stream {
            BitTorrentStream::Tcp(stream) => {
                PeerConnection::new_tcp(id, remote_socket, stream, incoming_buffer)
            }
            BitTorrentStream::Utp(_stream) => unimplemented!(),
        }
    }
    pub fn id(&self) -> &[u8; 20] {
        &self.id
    }
}

impl PeerConnection {
    fn new_tcp(
        id: [u8; 20],
        remote_socket: SocketAddr,
        stream: TcpStream,
        incoming_buffer: Arc<Mutex<Queue<Message>>>,
    ) -> Self {
        let (reader, writer) = stream.into_split();

        let reader_task = tokio::spawn(future)
        Self {
            id,
            choke_interest_data: 0,
            remote_socket,
            stream: writer.into(),
            close_at: Arc::new(Mutex::new(SystemTime::now().add(Duration::from_mins(2)))),
            reader_task: None,
        }
    }
    // pub fn new(
    //     id: [u8; 20],
    //     remote_socket: SocketAddr,
    //     stream: impl Into<BitTorrentStreamWriter>,
    // ) -> Self {
    //     Self {
    //         id,
    //         choke_interest_data: 0,
    //         remote_socket,
    //         stream: stream.into(),
    //         close_at: Arc::new(Mutex::new(SystemTime::now().add(Duration::from_mins(2)))),
    //     }
    // }

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
}

async fn start_reader(reader: BitTorrentStreamReader, incoming_buffer: Arc<Mutex<Queue<Message>>>) {
    match reader{
        BitTorrentStreamReader::Tcp(mut reader) => {
            loop{
                let buf = vec![]
            }
        }
        BitTorrentStreamReader::Utp(_) => unimplemented!()
    }
}
