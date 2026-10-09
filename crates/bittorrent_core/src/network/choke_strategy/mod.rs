use tokio::sync::mpsc;

use crate::util::Cancel;

struct PeerInfoStream {}

enum Instruction {
    Peer {
        id: [u8; 20],
        instruction: PeerInstruction,
    },
    Request {
        is_cancel: bool,
        piece: usize,
        begin: usize,
        size: usize,
    },
    Internal(InternalInstruction),
}

enum PeerInstruction {
    Choke,
    Unchoke,
    FlipChoke,
    Interest,
    Uninterest,
    FlipInterest,
}
enum InternalInstruction {
    SyncAll,
}

trait ChokeStrategy {
    type Activated: Cancel;

    fn start(self, info_stream: PeerInfoStream) -> (Self::Activated, mpsc::Receiver<Instruction>);
}
