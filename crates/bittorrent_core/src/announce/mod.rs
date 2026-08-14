use std::{fmt::Display, net::SocketAddr};

use crate::{announce::AnnounceEvent::*, session::SessionData};

pub mod http;
pub mod opts;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AnnounceEvent {
    Started,
    Completed,
    Stopped,
    RegularInterval,
}

pub struct Peer {
    id: Vec<u8>,
    socket: SocketAddr,
}

#[async_trait::async_trait]
pub trait Announce {
    async fn announce(
        &mut self,
        session_data: &SessionData,
        opts: AnnounceEvent,
    ) -> Result<Vec<Peer>, AnnounceError>;
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AnnounceError(Vec<u8>);

impl Display for AnnounceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(&self.0))
    }
}

impl AnnounceEvent {
    pub fn as_str(self) -> Option<&'static str> {
        match self {
            Started => Some("started"),
            Completed => Some("completed"),
            Stopped => Some("stopped"),
            RegularInterval => None,
        }
    }
}
