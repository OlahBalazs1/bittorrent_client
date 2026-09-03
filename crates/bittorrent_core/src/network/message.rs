use std::collections::VecDeque;

use crate::bitfield::Bitfield;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    KeepAlive,
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have(usize),
    Bitfield(Bitfield),
    Request {
        index: usize,
        begin: usize,
        length: usize,
    },
    Piece {
        index: usize,
        begin: usize,
        block: Vec<u8>,
    },
    Cancel {
        index: usize,
        begin: usize,
        length: usize,
    },
}
