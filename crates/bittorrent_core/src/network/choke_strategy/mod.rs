mod generous;
use tokio::sync::mpsc;

use crate::{bitfield::Bitfield, pieces::BlockDescriptor, util::Cancel};

pub(super) struct ChokeStrategyOutput {
    pub(super) instructions: mpsc::Receiver<Instruction>,
}

pub(super) enum StrategyInstruction {
    // strategy needs to choke someone
    TooGenerous,
    TooManyOutgoingRequests,
    SendHeartbeat,
}

pub(super) struct PeerInfo {
    pub(super) id: [u8; 20],
    pub(super) typ: PeerInfoType,
}

pub(super) enum PeerInfoType {
    FullInfo { bit_field: Bitfield, choke_info: u8 },
    Bitfield(Bitfield),
    Have(usize),
    Sent(BlockDescriptor),
    Canceled(BlockDescriptor),
    Disconnected,
}

pub(super) enum Instruction {
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
    Heartbeat,
}

pub(super) enum PeerInstruction {
    Choke,
    Unchoke,
    FlipChoke,
    Interest,
    Uninterest,
    FlipInterest,
}
pub(super) enum InternalInstruction {
    SyncAll,
}

pub(super) trait ChokeStrategy {
    type Activated: Cancel;

    fn start(
        self,
        instruction_stream: mpsc::Receiver<StrategyInstruction>,
        info_stream: mpsc::Receiver<PeerInfo>,
    ) -> (Self::Activated, ChokeStrategyOutput);
}
