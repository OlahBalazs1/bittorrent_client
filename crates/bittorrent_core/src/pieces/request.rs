use thiserror::Error;
use tokio::{
    sync::oneshot::{self},
    task::JoinHandle,
};

use crate::pieces::{Block, BlockDescriptor};

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
    block: BlockDescriptor,

    dst: Option<oneshot::Sender<Block>>,
}

impl BlockRequest {
    pub(crate) fn new(block: BlockDescriptor, return_path: oneshot::Sender<Block>) -> Self {
        Self {
            block,
            dst: Some(return_path),
        }
    }
    pub(crate) fn fulfill(&mut self, bytes: Box<[u8]>) -> Result<(), BlockFulfillError> {
        let Self { block, dst: _ } = self;
        let BlockDescriptor {
            piece,
            begin,
            length,
        } = *block;
        let sender = self.dst.take().ok_or(BlockFulfillError::AlreadyFulfilled)?;
        if bytes.len() != length {
            return Err(BlockFulfillError::WrongLength);
        }
        let block = Block {
            piece: piece,
            begin: begin,
            data: bytes,
        };
        Ok(sender
            .send(block)
            .map_err(|_| BlockFulfillError::Cancelled)?)
    }

    pub(crate) fn piece(&self) -> usize {
        self.block.piece
    }
    pub(crate) fn begin(&self) -> usize {
        self.block.begin
    }
    pub(crate) fn length(&self) -> usize {
        self.block.length
    }
}
