use std::sync::Arc;

use tokio::sync::mpsc::Receiver;

use crate::{
    metainfo::Metainfo,
    pieces::{Block, BlockRequest},
};

pub struct FsOptions {}

pub(super) struct FsHandlerEvents {
    piece_completed: Receiver<usize>,
}

pub(super) struct FsHandler {}

impl FsHandler {
    pub(crate) async fn new(
        metainfo: &Metainfo,
        fs_options: FsOptions,
    ) -> (Arc<Self>, FsHandlerEvents) {
        todo!()
    }
    pub(crate) async fn write_block(&self, block: Block) {
        todo!()
    }

    pub(crate) async fn handle_request(&self, request: BlockRequest) {
        todo!()
    }
}
