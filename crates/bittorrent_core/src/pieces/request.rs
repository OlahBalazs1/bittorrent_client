use thiserror::Error;
use tokio::sync::oneshot::{self};

use crate::pieces::Block;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Error)]
pub(crate) enum PieceRequestError {
    #[error("Request has already been fulfilled.")]
    AlreadyFulfilled,
    #[error("Byte slice sent to fulfill request is either too long or too short.")]
    WrongLength,
    #[error("Unknown error in channel.")]
    Unknown,
}

pub(crate) struct BlockRequest {
    piece: usize,
    begin: usize,
    length: usize,

    dst: Option<oneshot::Sender<Block>>,
}

impl BlockRequest {
    pub(crate) fn new(
        piece: usize,
        begin: usize,
        length: usize,
        return_path: oneshot::Sender<Block>,
    ) -> Self {
        Self {
            piece,
            begin,
            length,
            dst: Some(return_path),
        }
    }
    pub(crate) fn fulfill(&mut self, bytes: Vec<u8>) -> Result<(), PieceRequestError> {
        let sender = self.dst.take().ok_or(PieceRequestError::AlreadyFulfilled)?;
        if bytes.len() != self.length {
            return Err(PieceRequestError::WrongLength);
        }
        let block = Block {
            piece: self.piece,
            begin: self.begin,
            data: bytes,
        };
        Ok(sender.send(block).map_err(|_| PieceRequestError::Unknown)?)
    }
}
