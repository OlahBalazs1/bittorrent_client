use std::collections::HashMap;

use bencode::{Token, parse_bencode};
use sha1::{Digest, Sha1};

// TODO: Multitracker support (https://www.bittorrent.org/beps/bep_0012.html)
#[derive(Debug)]
pub struct Metainfo {
    pub announce: String,
    pub info: InfoDict,
    pub info_hash: [u8; 20],
}

impl Metainfo {
    pub fn parse(bencode: &[u8]) -> Self {
        let mut parsed = parse_bencode(bencode).unwrap().cast_dictionary().unwrap();
        let info = parsed.remove("info").unwrap();
        let info_hash = &info.clone().bencode();
        let info_hash = Sha1::digest(info_hash).into();

        Self {
            announce: parsed
                .remove("announce")
                .unwrap()
                .clone()
                .cast_string()
                .unwrap()
                .try_into()
                .unwrap(),
            info: InfoDict::new(info.cast_dictionary().unwrap()),
            info_hash,
        }
    }

    pub fn announce(&self) -> &str {
        &self.announce
    }
}

#[derive(Debug)]
pub struct InfoDict {
    name: String,
    piece_length: usize,
    pieces: Vec<[u8; 20]>,
    files: Option<Vec<File>>,
    length: Option<usize>,
}

impl InfoDict {
    fn new(mut info_dict: HashMap<String, Token>) -> Self {
        let name =
            String::from_utf8(info_dict.remove("name").unwrap().cast_string().unwrap()).unwrap();
        let piece_length = info_dict
            .remove("piece length")
            .unwrap()
            .cast_int()
            .unwrap()
            .try_into()
            .unwrap();

        let pieces = info_dict
            .remove("pieces")
            .unwrap()
            .cast_string()
            .unwrap()
            .array_windows::<20>()
            .step_by(20)
            .cloned()
            .collect::<Vec<_>>();

        let files = info_dict.remove("files").map(|e| {
            e.cast_list()
                .unwrap()
                .into_iter()
                .map(|e| File::new(e.cast_dictionary().unwrap()))
                .collect()
        });
        let length = info_dict
            .remove("length")
            .map(|e| e.cast_int().unwrap().try_into().unwrap());
        if files.is_some() == length.is_some() {
            panic!("Torrent file has both files and length, or neither");
        }

        Self {
            name,
            piece_length,
            pieces,
            files,
            length,
        }
    }
}

#[derive(Debug)]
struct File {
    length: usize,
    path: Vec<String>,
}

impl File {
    fn new(mut file_dict: HashMap<String, Token>) -> Self {
        let parsed_path = file_dict
            .remove("path")
            .unwrap()
            .cast_list()
            .unwrap()
            .into_iter()
            .map(|e| String::from_utf8(e.cast_string().unwrap()).unwrap())
            .collect();
        Self {
            length: file_dict
                .remove("length")
                .unwrap()
                .cast_int()
                .unwrap()
                .try_into()
                .unwrap(),
            path: parsed_path,
        }
    }
}
