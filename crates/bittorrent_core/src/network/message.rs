use crate::{
    bitfield::Bitfield,
    pieces::{Block, BlockDescriptor},
};

#[derive(Debug, Clone)]
pub enum Message {
    KeepAlive,
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have(usize),
    Bitfield(Bitfield),
    Request(BlockDescriptor),
    Piece(Block),
    Cancel(BlockDescriptor),
}
