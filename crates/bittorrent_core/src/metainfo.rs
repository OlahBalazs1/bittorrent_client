use bencode::{Value, from_bytes, to_bytes};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

// TODO: Multitracker support (https://www.bittorrent.org/beps/bep_0012.html)
#[derive(Debug, Deserialize, Serialize)]
pub struct Metainfo {
    pub announce: String,
    pub info: InfoDict,
    #[serde(skip)]
    pub info_hash: [u8; 20],
}

impl Metainfo {
    pub fn parse(bencode: &[u8]) -> Self {
        #[derive(Deserialize)]
        struct InfoUnwrap {
            info: Value,
        }
        let mut parsed: Self = from_bytes(bencode).unwrap();
        let as_value: InfoUnwrap = from_bytes(bencode).unwrap();
        parsed.info_hash = Sha1::digest(to_bytes(&as_value.info).unwrap()).into();
        parsed
    }

    pub fn announce(&self) -> &str {
        &self.announce
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct InfoDict {
    #[serde(rename = "piece length")]
    piece_length: usize,
    #[serde(with = "serde_bytes")]
    pieces: Vec<u8>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    length: Option<usize>,
    #[serde(default)]
    files: Option<Vec<File>>,
}

#[derive(Debug, Deserialize, Serialize)]
struct File {
    length: usize,
    path: Vec<String>,
}
