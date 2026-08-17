use std::{
    fmt::{Display, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr},
};

use bencode::{Token, parse_bencode};
pub use reqwest::Error as NetworkError;
use thiserror::Error;
use url_encode::url_encode;

use crate::{announce::AnnounceEvent::*, session::SessionData};

pub mod http;

#[derive(Debug, Default)]
pub struct AnnounceOpts {
    pub info_hash: [u8; 20],
    pub peer_id: [u8; 20],
    pub ip: Option<Ipv4Addr>,
    pub port: u16,
    pub uploaded: usize,
    pub downloaded: usize,
    pub left: usize,
    pub event: AnnounceEvent,
    pub compact: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum AnnounceEvent {
    Started,
    Completed,
    Stopped,
    #[default]
    RegularInterval,
}

#[derive(Debug)]
pub struct Peer {
    id: Vec<u8>,
    socket: SocketAddr,
}

#[derive(Debug, Error)]
#[error("Parsed token wasn't a bencoded list of peers")]
pub struct NonCompactPeerListError;

#[async_trait::async_trait]
pub trait Announce {
    async fn announce(
        &mut self,
        session_data: &SessionData,
        opts: AnnounceOpts,
    ) -> Result<Vec<Peer>, AnnounceError>;
}

#[derive(Debug, Error)]
pub enum AnnounceError {
    Network(NetworkError),
    BitTorrent(Vec<u8>),
    Unknown,
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

impl Display for AnnounceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Network(err) => write!(f, "{}", err).unwrap(),
            Self::BitTorrent(err) => {
                let as_utf = String::from_utf8_lossy(err);
                write!(f, "{}", as_utf).unwrap()
            }
            Self::Unknown => write!(f, "Unknown error").unwrap(),
        }
        Ok(())
    }
}

impl AnnounceOpts {
    pub fn to_uri_query_parameters(&self) -> String {
        let mut out = String::new();
        write!(out, "info_hash={}&", url_encode(&self.info_hash)).unwrap();
        write!(out, "peer_id={}&", url_encode(&self.peer_id)).unwrap();
        if let Some(ip) = self.ip {
            write!(out, "ip={ip}&").unwrap()
        }
        write!(out, "port={}&", self.port).unwrap();
        write!(out, "uploaded={}&", self.uploaded).unwrap();
        write!(out, "downloaded={}&", self.downloaded).unwrap();
        write!(out, "left={}&", self.left).unwrap();
        if let Some(event) = self.event.as_str() {
            write!(out, "event={}&", event).unwrap();
        }
        write!(out, "compact={}&", if self.compact { 1 } else { 0 }).unwrap();
        out
    }
}

fn parse_noncompact_peerlist(list: &[u8]) -> Option<Vec<Peer>> {
    let list = parse_bencode(list).ok()?;
    let list = list.cast_list()?;
    let mut out = Vec::with_capacity(list.len());
    for token in list {
        let mut dict = token.cast_dictionary()?;
        let id = dict.remove("id")?.cast_string()?;

        let ip: String = dict.remove("ip")?.cast_string()?.try_into().ok()?;

        let port: u16 = dict.remove("port")?.cast_int()?.try_into().ok()?;

        let socket = SocketAddr::new(ip.parse::<IpAddr>().ok()?, port);

        out.push(Peer { id, socket })
    }

    Some(out)
}

fn parse_peer_list(list: Vec<u8>) -> Option<Vec<Peer>> {
    if let Some(noncompact) = parse_noncompact_peerlist(&list) {
        Some(noncompact)
    } else {
        None
    }
}
