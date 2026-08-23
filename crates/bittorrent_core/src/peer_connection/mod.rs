use std::net::SocketAddr;

use crate::network::BitTorrentStream;
#[derive(Debug)]
pub struct Peer {
    pub(crate) id: Option<Vec<u8>>,
    pub(crate) socket: SocketAddr,
}

#[derive(Debug)]
pub struct PeerConnection {
    id: [u8; 20],
    choke_interest_data: u8,
    remote_socket: SocketAddr,
    stream: BitTorrentStream,
}

impl PeerConnection {
    pub fn new(
        id: [u8; 20],
        remote_socket: SocketAddr,
        stream: impl Into<BitTorrentStream>,
    ) -> Self {
        Self {
            id,
            choke_interest_data: 0,
            remote_socket,
            stream: stream.into(),
        }
    }

    pub async fn send_handshake(&mut self, info_hash: &[u8; 20], peer_id: &[u8; 20]) {
        self.stream.send_handshake(info_hash, peer_id).await
    }
    pub fn id(&self) -> &[u8] {
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
