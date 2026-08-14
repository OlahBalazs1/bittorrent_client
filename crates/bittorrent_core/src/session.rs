use std::{
    fmt::Display,
    net::{TcpListener, UdpSocket},
};

use rand::{RngExt, rng};

use crate::metainfo::Metainfo;

pub struct Session {
    pub data: SessionData,
}

pub struct SessionData {
    pub metainfo: Metainfo,
    port: u16,

    peer_id: String,
}

impl Session {
    pub fn new(metainfo: Metainfo, protocol: Protocol) -> Self {
        Self {
            data: SessionData {
                metainfo,
                port: find_port(protocol).expect(&format!(
                    "No open {protocol} port in range 6882..=6889 found."
                )),
                peer_id: generate_peer_id(),
            },
        }
    }

    pub fn port(&self) -> u16 {
        self.data.port
    }

    pub fn peer_id(&self) -> &str {
        &self.data.peer_id
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Protocol {
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
