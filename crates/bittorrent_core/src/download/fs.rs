use tokio::sync::mpsc::Receiver;

pub(super) struct FsHandlerEvents {
    piece_completed: Receiver<usize>,
}
pub(super) struct FsHandler {}
