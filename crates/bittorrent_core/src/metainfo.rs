use std::collections::HashMap;

use bencode::{Token, parse_bencode};

// TODO: Multitracker support (https://www.bittorrent.org/beps/bep_0012.html)
#[derive(Debug)]
pub struct Metainfo {
    announce: String,
    info: HashMap<String, Token>,
}

impl Metainfo {
    pub fn parse(bencode: &[u8]) -> Self {
        let parsed = parse_bencode(bencode).unwrap().cast_dictionary().unwrap();

        Self {
            announce: parsed
                .get("announce")
                .unwrap()
                .clone()
                .cast_string()
                .unwrap()
                .try_into()
                .unwrap(),
            info: parsed
                .get("info")
                .unwrap()
                .clone()
                .cast_dictionary()
                .unwrap(),
        }
    }
}
