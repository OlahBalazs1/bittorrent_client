use std::io;

use tokio::{
    io::AsyncWriteExt,
    net::{TcpStream, UdpSocket},
};

#[derive(Debug)]
pub enum BitTorrentStream {
    Tcp(TcpStream),
    Utp(UdpSocket),
}

impl BitTorrentStream {
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
