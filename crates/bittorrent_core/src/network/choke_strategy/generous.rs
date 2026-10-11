use parking_lot::RwLock;
use std::{
    collections::{HashMap, HashSet, hash_map::Entry},
    marker::PhantomData,
    process::Output,
    sync::{Arc, atomic::AtomicU32},
};

use tokio::sync::{Notify, mpsc};

use crate::{
    bitfield::Bitfield,
    network::choke_strategy::{
        ChokeStrategy, ChokeStrategyOutput, PeerInfo, PeerInfoType, StrategyInstruction,
    },
    util::{Cancel, CancelNotify},
};

/// A simple strategy intended to finally download a file and to test the client
pub(super) struct GenerousStrategy {
    bitfields: HashMap<[u8; 20], Bitfield>,
    is_choking: HashSet<[u8; 20]>,
    am_choking: HashSet<[u8; 20]>,
    am_interested: HashSet<[u8; 20]>,
}

impl GenerousStrategy {
    fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
}

impl Default for GenerousStrategy {
    fn default() -> Self {
        Self {
            bitfields: HashMap::default(),
            is_choking: HashSet::default(),
            am_choking: HashSet::default(),
            am_interested: HashSet::default(),
        }
    }
}
// it may be beneficial to apply a generous alignment to this
// or to box every field (to prevent false sharing)
// although I'll only do that once I'm doing benchmarks
struct RunningGenerousStrategy {
    bitfields: RwLock<HashMap<[u8; 20], Bitfield>>,
    is_choking: RwLock<HashSet<[u8; 20]>>,
    am_choking: RwLock<HashSet<[u8; 20]>>,

    interested_in: RwLock<HashSet<usize>>,

    piece_download_cap: AtomicU32,
    currently_downloading: AtomicU32,
}

impl ChokeStrategy for GenerousStrategy {
    type Activated = Arc<CancelNotify>;

    fn start(
        self,
        instruction_stream: tokio::sync::mpsc::Receiver<super::StrategyInstruction>,
        info_stream: tokio::sync::mpsc::Receiver<super::PeerInfo>,
    ) -> (Self::Activated, super::ChokeStrategyOutput) {
        let GenerousStrategy {
            bitfields,
            is_choking,
            am_choking,
            am_interested,
        } = self;
        let canceller = Arc::new(CancelNotify(Notify::const_new()));
        let (instruction_send, instruction_recv) = mpsc::channel(128);

        let running = Arc::new(RunningGenerousStrategy {
            bitfields: parking_lot::RwLock::new(bitfields),
            is_choking: parking_lot::RwLock::new(is_choking),
            am_choking: parking_lot::RwLock::new(am_choking),

            interested_in: parking_lot::RwLock::new(HashSet::<usize>::new()),
            piece_download_cap: AtomicU32::new(1000),
            currently_downloading: AtomicU32::new(0),
        });
        // info task
        {
            let canceller = Arc::clone(&canceller);
            let mut info_stream = info_stream;
            let running = Arc::clone(&running);
            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _ = canceller.0.notified() => {
                            break;
                        }
                    info = info_stream.recv() => {
                    let Some(info) = info else {
                        canceller.0.notify_waiters();
                        break;
                    };
                    let PeerInfo { id, typ } = info;
                    match typ {
                        // 0001 => is_choking
                        // 0010 => is_interested
                        // 0100 => am_choking
                        // 1000 => am_interested
                        PeerInfoType::FullInfo {
                            bit_field,
                            choke_info,
                        } => {
                            running.bitfields.write().insert(id, bit_field);
                            if choke_info & 0b1 != 0 {
                                running.is_choking.write().insert(id);
                            }
                            if choke_info & 0b100 != 0 {
                                running.am_choking.write().insert(id);
                            }
                        }

                        PeerInfoType::Bitfield(field) => {
                            running.bitfields.write().insert(id, field);
                        }
                        PeerInfoType::Have(piece) => match running.bitfields.write().entry(id) {
                            Entry::Occupied(mut entry) => {
                                let _ = entry.get_mut().set(piece, true);
                            }
                            Entry::Vacant(_) => log::error!(
                                "GenerousStrategy received a Have message for an unknown peer",
                            ),
                        },
                        PeerInfoType::Disconnected => {
                            running.bitfields.write().remove(&id);
                            running.is_choking.write().remove(&id);
                            running.am_choking.write().remove(&id);
                        }
                        PeerInfoType::Sent(_) => {}
                        PeerInfoType::Canceled(_) => {}
                    }
                    }

                    }
                }
            });
        }

        // TODO: Pick pieces to download

        let output = ChokeStrategyOutput {
            instructions: instruction_recv,
        };

        (canceller, output)
    }
}
