use std::{
    fmt::{Display, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4},
};

use crate::{announce::AnnounceError::Unknown, peer_connection::Peer};
use bencode::Token;
pub use reqwest::Error as NetworkError;
use thiserror::Error;
use url_encode::url_encode;

use crate::{announce::AnnounceEvent::*, session::SessionData};

pub mod http;

pub struct AnnounceResponse {
    interval: u32,
    min_interval: Option<u32>,

    peers: Vec<Peer>,
    complete: u32,
    incomplete: u32,

    tracker_id: Option<Vec<u8>>,
}

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

#[async_trait::async_trait]
pub trait Announce {
    async fn announce(
        &mut self,
        session_data: &SessionData,
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

fn parse_noncompact_peerlist(list: Vec<Token>) -> Option<Vec<Peer>> {
    let mut out = Vec::with_capacity(list.len());
    for token in list {
        let mut dict = token.cast_dictionary()?;
        let id = dict.remove("id")?.cast_string()?;

        let ip: String = dict.remove("ip")?.cast_string()?.try_into().ok()?;

        let port: u16 = dict.remove("port")?.cast_int()?.try_into().ok()?;

        let socket = SocketAddr::new(ip.parse::<IpAddr>().ok()?, port);

        out.push(Peer {
            id: Some(id),
            socket,
        })
    }

    Some(out)
}

fn parse_compact_peerlist(list: Vec<u8>) -> Option<Vec<Peer>> {
    let mut peers = Vec::with_capacity(list.len() / 6);
    for peer in list.chunks(6) {
        let ip = Ipv4Addr::from_octets(peer[0..4].try_into().unwrap());
        let port = u16::from_le_bytes(peer[4..6].try_into().unwrap());

        let socket = SocketAddrV4::new(ip, port);

        peers.push(Peer {
            id: None,
            socket: socket.into(),
        });
    }

    Some(peers)
}

fn parse_peer_list(list: Token) -> Option<Vec<Peer>> {
    if let Some(noncompact) = list
        .cast_list_ref()
        .and_then(|e| parse_noncompact_peerlist(e.clone()))
    {
        Some(noncompact)
    } else if let Some(compact) = list
        .cast_string_ref()
        .and_then(|e| parse_compact_peerlist(e.to_vec()))
    {
        Some(compact)
    } else {
        None
    }
}

impl AnnounceResponse {
    fn parse_bdecoded(token: Token) -> Result<Self, AnnounceError> {
        let mut map = token.cast_dictionary().ok_or(AnnounceError::Unknown)?;
        if let Some(error) = map.remove("failure") {
            return Err(AnnounceError::BitTorrent(error.cast_string().unwrap()));
        }

        let peers = parse_peer_list(map.remove("peers").ok_or(AnnounceError::Unknown)?)
            .ok_or(AnnounceError::Unknown)?;
        let complete = map
            .remove("complete")
            .ok_or(Unknown)?
            .cast_int()
            .ok_or(Unknown)?
            .try_into()
            .map_err(|_| Unknown)?;
        let incomplete = map
            .remove("incomplete")
            .ok_or(Unknown)?
            .cast_int()
            .ok_or(Unknown)?
            .try_into()
            .map_err(|_| Unknown)?;

        let interval = map
            .remove("interval")
            .ok_or(Unknown)?
            .cast_int()
            .ok_or(Unknown)?
            .try_into()
            .map_err(|_| Unknown)?;
        let min_interval: Option<u32> = map
            .remove("min interval")
            .and_then(|e| e.cast_int())
            .map(|e| e.try_into().expect("Min interval should fit into a u32"));

        let tracker_id = map.remove("tracker id").and_then(|e| e.cast_string());
        Ok(Self {
            tracker_id,
            peers,
            complete,
            incomplete,
            interval,
            min_interval,
        })
    }
}
