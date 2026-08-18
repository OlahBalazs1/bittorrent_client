use std::net::SocketAddr;
#[derive(Debug)]
pub struct Peer {
    pub(crate) id: Option<Vec<u8>>,
    pub(crate) socket: SocketAddr,
}
