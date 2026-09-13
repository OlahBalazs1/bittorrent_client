use thiserror::Error;
use tokio::{
    sync::oneshot::{self},
    task::JoinHandle,
};

use crate::pieces::Block;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Error)]
pub(crate) enum BlockFulfillError {
    #[error("Request has already been fulfilled.")]
    AlreadyFulfilled,
    #[error("Byte slice sent to fulfill request is either too long or too short.")]
    WrongLength,
    #[error("Unknown error in channel.")]
    Cancelled,
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
    pub(crate) fn fulfill(&mut self, bytes: Box<[u8]>) -> Result<(), BlockFulfillError> {
        let sender = self.dst.take().ok_or(BlockFulfillError::AlreadyFulfilled)?;
        if bytes.len() != self.length {
            return Err(BlockFulfillError::WrongLength);
        }
        let block = Block {
            piece: self.piece,
            begin: self.begin,
            data: bytes,
        };
        Ok(sender
            .send(block)
            .map_err(|_| BlockFulfillError::Cancelled)?)
    }
}
