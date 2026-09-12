use super::network::peer_connection::Peer;
use std::{
    fmt::{Display, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4},
};

use crate::announce::AnnounceError::*;
use bencode::from_bytes;
pub use reqwest::Error as NetworkError;
use serde::Deserialize;
use thiserror::Error;
use url_encode::url_encode;

use crate::announce::AnnounceEvent::*;

pub mod http;

#[derive(Deserialize)]
pub struct NoncompactPeer {
    #[serde(with = "serde_bytes")]
    #[serde(alias = "peer id")]
    id: Vec<u8>,
    ip: IpAddr,
    port: u16,
}

#[derive(Deserialize)]
pub struct NoncompactPeerlist {
    peers: Vec<NoncompactPeer>,
}

#[derive(Deserialize)]
pub struct CompactPeerlist {
    #[serde(with = "serde_bytes")]
    peers: Vec<u8>,
}

impl NoncompactPeerlist {
    fn to_peerlist(self) -> Vec<Peer> {
        self.peers
            .into_iter()
            .filter_map(|e| {
                Some(Peer {
                    id: Some(e.id.try_into().ok()?),
                    socket: SocketAddr::new(e.ip, e.port),
                })
            })
            .collect()
    }
}

impl CompactPeerlist {
    fn to_peerlist(self) -> Vec<Peer> {
        let mut peers = Vec::with_capacity(self.peers.len() / 6);
        for peer in self.peers.chunks(6) {
            let ip = u32::from_be_bytes(peer[0..4].try_into().unwrap());
            let port = u16::from_be_bytes(peer[4..6].try_into().unwrap());

            let socket = SocketAddrV4::new(Ipv4Addr::from_bits(ip), port);

            peers.push(Peer {
                id: None,
                socket: socket.into(),
            });
        }

        peers
    }
}

#[derive(Debug, Deserialize)]
pub struct AnnounceResponse {
    pub(crate) interval: u32,
    #[serde(rename = "min interval")]
    pub(crate) min_interval: Option<u32>,

    #[serde(skip)]
    pub(crate) peers: Vec<Peer>,
    pub(crate) complete: u32,
    pub(crate) incomplete: u32,

    #[serde(rename = "tracker id")]
    pub(crate) tracker_id: Option<Vec<u8>>,
}

#[derive(Debug, Default)]
pub struct AnnounceOpts {
    pub info_hash: [u8; 20],
    pub peer_id: [u8; 20],
    pub ip: Option<IpAddr>,
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

#[async_trait::async_trait]
pub trait Announce {
    async fn announce(
        &mut self,
        announce_url: &str,
        opts: AnnounceOpts,
    ) -> Result<AnnounceResponse, AnnounceError>;
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

impl AnnounceResponse {
    fn from_bytes(response: &[u8]) -> Result<Self, AnnounceError> {
        #[derive(Deserialize)]
        struct Failure {
            #[serde(with = "serde_bytes")]
            #[serde(rename = "failure reason")]
            reason: Vec<u8>,
        }
        if let Some(failure) = from_bytes::<Failure>(response).ok() {
            return Err(BitTorrent(failure.reason));
        }
        let mut parsed: Self = from_bytes(response).map_err(|_| AnnounceError::Unknown)?;

        if let Some(noncompact) = from_bytes::<NoncompactPeerlist>(response).ok() {
            parsed.peers = noncompact.to_peerlist();
        } else if let Some(compact) = from_bytes::<CompactPeerlist>(response).ok() {
            parsed.peers = compact.to_peerlist();
        } else {
            return Err(AnnounceError::Unknown);
        }
        Ok(parsed)
    }
}
