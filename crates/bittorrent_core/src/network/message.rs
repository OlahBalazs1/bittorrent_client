use std::collections::VecDeque;

use crate::bitfield::Bitfield;

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

pub struct TcpMessageParser {
    unparsed_bytes: Vec<u8>,
    unread_messages: VecDeque<Message>,
}


