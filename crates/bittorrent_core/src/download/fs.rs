use tokio::sync::mpsc::Receiver;

use crate::pieces::Block;

pub(super) struct FsHandlerEvents {
    piece_completed: Receiver<usize>,
}
pub(super) struct FsHandler {}

impl FsHandler {
    // uniquely owned by Download, so no Arc<Self>
    pub(crate) async fn new() -> (Self, FsHandlerEvents) {
        todo!()
    }
    pub(crate) async fn write_block(&self, block: Block) {
        todo!()
    }
}
