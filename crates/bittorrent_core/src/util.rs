use std::io::{self, Write};

use log::info;
use rand::{fill, random, random_iter, random_range};

const IDENTIFIER: &'static [u8] = b"-CW";
const VERSION_NUMBER: &'static [u8; 4] = b"0001";

pub fn generate_peer_id() -> [u8; 20] {
    let mut id: [u8; 20] = [0; 20];
    id[0..3].clone_from_slice(IDENTIFIER);
    id[3..7].clone_from_slice(VERSION_NUMBER);
    for index in 7..20 {
        // keep everything nice and mostly ASCII
        id[index] = match random_range(0..3u8) {
            0 => random_range(b'a'..=b'z'),
            1 => random_range(b'A'..=b'Z'),
            2 => random_range(b'0'..=b'9'),
            _ => unreachable!(),
        }
    }
    info!("generated id: {}", String::from_utf8_lossy(&id));
    id
}
