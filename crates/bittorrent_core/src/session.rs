use std::{
    fmt::Display,
    net::{TcpListener, UdpSocket},
};

use rand::{RngExt, random, rng};

use crate::{
    announce::{self, Announce, AnnounceOpts, Peer},
    metainfo::Metainfo,
};

#[derive(Default)]
pub struct SessionBuilder {
    protocol: Protocol,
}

impl SessionBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_protocol(self, protocol: Protocol) -> Self {
        Self { protocol, ..self }
    }
    pub fn build<A: Announce + 'static>(self, metainfo: Metainfo, announcer: A) -> Session {
        Session::new(metainfo, self.protocol, Box::new(announcer))
    }
}

pub struct Session {
    pub data: SessionData,
    announcer: Box<dyn Announce>,
}

pub struct SessionData {
    pub metainfo: Metainfo,
    port: u16,

    peer_id: [u8; 20],
}

impl Session {
    fn new(metainfo: Metainfo, protocol: Protocol, announcer: Box<dyn Announce>) -> Self {
        Self {
            data: SessionData {
                metainfo,
                port: find_port(protocol).expect(&format!(
                    "No open {protocol} port in range 6882..=6889 found."
                )),
                peer_id: random(),
            },
            announcer,
        }
    }

    pub async fn announce(&mut self) -> Vec<Peer> {
        self.announcer
            .announce(
                &self.data,
                AnnounceOpts {
                    info_hash: self.data.metainfo.info_hash,
                    peer_id: *self.peer_id(),
                    downloaded: 0,
                    left: 0,
                    uploaded: 0,
                    ip: None,
                    port: self.port(),
                    compact: false,
                    event: announce::AnnounceEvent::Started,
                },
            )
            .await
            .unwrap()
    }

    pub fn port(&self) -> u16 {
        self.data.port
    }

    pub fn peer_id(&self) -> &[u8; 20] {
        &self.data.peer_id
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum Protocol {
    #[default]
    TCP,
    UDP,
}

impl Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Protocol::TCP => write!(f, "TCP"),
            Protocol::UDP => write!(f, "UDP"),
        }
    }
}

fn find_port(protocol: Protocol) -> Option<u16> {
    for i in 6882..=6889 {
        match protocol {
            Protocol::TCP if TcpListener::bind(("127.0.0.1", i)).is_ok() => return Some(i),
            Protocol::UDP if UdpSocket::bind(("127.0.0.1", i)).is_ok() => return Some(i),
            _ => {}
        }
    }
    None
}

fn generate_peer_id() -> String {
    let mut out = String::new();
    let mut rng = rng();
    for _ in 0..20 {
        out.push(match rng.random_range(0..3) {
            // lowercase alphabet
            0 => rng.random_range('a'..='z'),
            // uppercase alphabet
            1 => rng.random_range('A'..='B'),
            // number
            2 => rng.random_range('0'..='9'),
            _ => unreachable!(),
        });
    }
    out
}
