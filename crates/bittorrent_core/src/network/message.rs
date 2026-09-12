use crate::{bitfield::Bitfield, pieces::Block};

#[derive(Debug, Clone)]
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
    Piece(Block),
    Cancel {
        piece: usize,
        begin: usize,
        length: usize,
    },
}
