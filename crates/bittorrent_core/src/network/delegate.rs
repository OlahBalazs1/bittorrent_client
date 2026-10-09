use std::{
    collections::HashMap,
    ops::Deref,
    sync::{
        Arc, Weak,
        nonpoison::{self, Mutex},
    },
};

use tokio::{
    sync::{Mutex as TokioMutex, mpsc},
    task::{JoinHandle, JoinSet},
};

use super::peer_connection::PeerConnection;

use crate::{
    announce::{AnnounceError, AnnounceResponse},
    network::{
        ExtendedOpts, MinimalOpts, NetworkContext, delegate,
        peer_connection::{BubbledMessage, InactivePeerConnection, PeerConnectionIo},
    },
    pieces::{Block, BlockRequest},
    util::{Cancel, CancelGuard},
};

pub(crate) struct NetworkDelegateIo {
    pub(crate) piece_requests: mpsc::Receiver<BlockRequest>,
    pub(crate) incoming_blocks: mpsc::Receiver<Block>,
}

// in charge of:
// - announces
// - making connections
// - holding connections
// - handling connections (interest, choking, etc)
// - asking for pieces
// - relaying requests for pieces
pub(crate) struct NetworkDelegate {
    ctx: Weak<NetworkContext>,
    info_hash: [u8; 20],
    peer_id: [u8; 20],

    last_announce: Mutex<Option<AnnounceResponse>>,

    request_send: mpsc::Sender<BlockRequest>,
    incoming_blocks: mpsc::Sender<Block>,

    tasks: Mutex<JoinSet<()>>,

    // { peer_id: index_in_peer_connections}
    connected_peers: Mutex<HashMap<[u8; 20], CancelGuard<Arc<PeerConnection>>>>,
}

impl NetworkDelegate {
    pub(super) fn new(
        ctx: Arc<NetworkContext>,
        info_hash: [u8; 20],
        peer_id: [u8; 20],
    ) -> (Arc<Self>, NetworkDelegateIo) {
        let (request_send, request_recv) = mpsc::channel::<BlockRequest>(128);
        let (block_send, block_recv) = mpsc::channel::<Block>(128);

        let delegate = Self {
            ctx: Arc::downgrade(&ctx),
            last_announce: Mutex::new(None),
            connected_peers: Default::default(),
            info_hash,
            peer_id,
            request_send,
            incoming_blocks: block_send,

            tasks: Mutex::new(JoinSet::new()),
        };

        (
            Arc::new(delegate),
            NetworkDelegateIo {
                incoming_blocks: block_recv,
                piece_requests: request_recv,
            },
        )
    }

    // convenience function for upgrading ctx
    async fn get_ctx(&self) -> Option<Arc<NetworkContext>> {
        let Some(ctx) = self.ctx.upgrade() else {
            self.cancel().await;
            return None;
        };
        Some(ctx)
    }

    pub(crate) async fn announce(
        &self,
        announce_url: &str,
        opts: MinimalOpts,
    ) -> Result<(), AnnounceError> {
        let MinimalOpts {
            uploaded,
            downloaded,
            left,
            event,
            compact,
        } = opts;

        let Some(ctx) = self.get_ctx().await else {
            return Err(AnnounceError::Unknown);
        };
        let announce_response = ctx
            .announce(
                announce_url,
                ExtendedOpts {
                    info_hash: self.info_hash,
                    peer_id: self.peer_id,
                    uploaded,
                    downloaded,
                    left,
                    event,
                    compact,
                },
            )
            .await?;
        *self.last_announce.lock() = Some(announce_response);
        Ok(())
    }

    async fn start_automatic_reannounce_task(self: Arc<Self>) {
        todo!()
    }
    pub(crate) async fn register_connection(self: Arc<Self>, connection: InactivePeerConnection) {
        if self
            .connected_peers
            .lock()
            .contains_key(connection.id() as &[u8])
        {
            return;
        }
        let Some(ctx) = self.get_ctx().await else {
            return;
        };
        let (connection, events) = connection.activate().await;
        let PeerConnectionIo {
            mut bubble_recv,
            on_shutdown,
        } = events;

        connection
            .send_handshake(&self.info_hash, &self.peer_id)
            .await;

        {
            let delegate = Arc::clone(&self);
            self.tasks.lock().spawn(async move {
                while let Some(message) = bubble_recv.recv().await {
                    match message {
                        BubbledMessage::Request(request) => {
                            // fail = Download closed its Receiver => Delegate not needed anymore
                            let Ok(_) = delegate.request_send.send(request).await else {
                                (&*delegate).cancel().await;
                                return;
                            };
                        }
                        BubbledMessage::Piece(block) => {
                            // fail = Download closed its Receiver => Delegate not needed anymore
                            let Ok(_) = delegate.incoming_blocks.send(block).await else {
                                (&*delegate).cancel().await;
                                return;
                            };
                        }

                        // useful for choking and request strategy
                        BubbledMessage::Have(_have) => todo!(),
                        // useful for choking and request strategy
                        BubbledMessage::Bitfield(_bitfield) => todo!(),
                    }
                }
            });
        }

        {
            let ctx = Arc::clone(&ctx);
            self.tasks.lock().spawn(async move {
                on_shutdown.notified().await;
                ctx.add_active_connections(-1);
            });
        }

        let id = *connection.id();
        ctx.add_active_connections(1);
        self.connected_peers
            .lock()
            .insert(id, CancelGuard::new(connection));
    }
}

#[async_trait::async_trait]
impl Cancel for NetworkDelegate {
    async fn cancel(&self) {
        self.tasks.lock().abort_all();

        let Some(ctx) = self.ctx.upgrade() else {
            return;
        };
        ctx.close_delegate(&self.peer_id).await
    }
}
