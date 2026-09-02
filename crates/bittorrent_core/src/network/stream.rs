use std::sync::Arc;

use tokio::{
    io::AsyncWriteExt,
    net::{TcpStream, UdpSocket, tcp},
};
#[derive(Debug)]
pub enum BitTorrentStream {
    Tcp(TcpStream),
    Utp(UdpSocket),
}

#[derive(Debug)]
pub enum BitTorrentStreamReader {
    Tcp(tcp::OwnedReadHalf),
    Utp(Arc<UdpSocket>),
}

#[derive(Debug)]
pub enum BitTorrentStreamWriter {
    Tcp(tcp::OwnedWriteHalf),
    Utp(Arc<UdpSocket>),
}

impl BitTorrentStream {
    pub fn into_split(self) -> (BitTorrentStreamReader, BitTorrentStreamWriter) {
        match self {
            Self::Tcp(stream) => {
                let (read, write) = stream.into_split();
                (
                    BitTorrentStreamReader::Tcp(read),
                    BitTorrentStreamWriter::Tcp(write),
                )
            }
            Self::Utp(stream) => {
                let arced = Arc::new(stream);

                (
                    BitTorrentStreamReader::Utp(Arc::clone(&arced)),
                    BitTorrentStreamWriter::Utp(arced),
                )
            }
        }
    }
}

impl BitTorrentStreamWriter {
    pub async fn send_handshake(&mut self, info_hash: &[u8; 20], peer_id: &[u8; 20]) {
        match self {
            Self::Tcp(stream) => {
                let mut message = Vec::<u8>::new();
                const PROTOCOL: &[u8; 19] = b"BitTorrent Protocol";

                message.push(PROTOCOL.len() as u8);
                message.extend_from_slice(PROTOCOL);
                message.extend_from_slice(info_hash);
                message.extend_from_slice(peer_id);

                stream.write_all(&message).await.unwrap();
            }
            Self::Utp(_) => unimplemented!(),
        }
    }
}

impl From<TcpStream> for BitTorrentStream {
    fn from(value: TcpStream) -> Self {
        Self::Tcp(value)
    }
}
impl From<UdpSocket> for BitTorrentStream {
    fn from(value: UdpSocket) -> Self {
        Self::Utp(value)
    }
}

impl From<tcp::OwnedWriteHalf> for BitTorrentStreamWriter {
    fn from(value: tcp::OwnedWriteHalf) -> Self {
        Self::Tcp(value)
    }
}

impl From<Arc<UdpSocket>> for BitTorrentStreamWriter {
    fn from(value: Arc<UdpSocket>) -> Self {
        Self::Utp(value)
    }
}
