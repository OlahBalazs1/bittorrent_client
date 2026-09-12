use std::{
    collections::HashMap,
    sync::{
        Arc,
        nonpoison::{self, Mutex},
    },
};

use tokio::{
    sync::{Mutex as TokioMutex, mpsc},
    task::JoinHandle,
};

use super::peer_connection::PeerConnection;

use crate::{
    announce::{AnnounceError, AnnounceResponse},
    network::{
        ExtendedOpts, MinimalOpts, NetworkContext, delegate,
        peer_connection::{BubbledMessage, InactivePeerConnection, PeerConnectionIo},
    },
    pieces::{Block, BlockRequest},
};

pub(crate) struct NetworkDelegateIo {
    piece_requests: mpsc::Receiver<BlockRequest>,
    incoming_blocks: mpsc::Receiver<Block>,
}

struct NetworkDelegateTasks {
    automatic_reannounce: Option<JoinHandle<()>>,
}

// in charge of:
// - announces
// - making connections
// - holding connections
// - handling connections (interest, choking, etc)
// - asking for pieces
// - relaying requests for pieces
pub(crate) struct NetworkDelegate {
    ctx: Arc<NetworkContext>,
    info_hash: [u8; 20],
    peer_id: [u8; 20],

    last_announce: Mutex<Option<AnnounceResponse>>,

    tasks: nonpoison::Mutex<NetworkDelegateTasks>,

    request_send: mpsc::Sender<BlockRequest>,
    incoming_blocks: mpsc::Sender<Block>,

    // { peer_id: index_in_peer_connections}
    connected_peers: Mutex<HashMap<[u8; 20], Arc<PeerConnection>>>,
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
            ctx,
            last_announce: Mutex::new(None),
            connected_peers: Default::default(),
            info_hash,
            peer_id,
            request_send,
            incoming_blocks: block_send,

            tasks: Mutex::new(NetworkDelegateTasks {
                automatic_reannounce: None,
            }),
        };

        (
            Arc::new(delegate),
            NetworkDelegateIo {
                incoming_blocks: block_recv,
                piece_requests: request_recv,
            },
        )
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
        let announce_response = self
            .ctx
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

    pub(crate) async fn shutdown(&self) {
        if let Some(reannounce) = self.tasks.lock().automatic_reannounce.take() {
            reannounce.abort();
        }
        self.ctx.close_delegate(&self.peer_id).await
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
            tokio::spawn(async move {
                while let Some(message) = bubble_recv.recv().await {
                    match message {
                        BubbledMessage::Request(request) => {
                            // fail = Download closed its Receiver => Delegate not needed anymore
                            let Ok(_) = delegate.request_send.send(request).await else {
                                delegate.shutdown().await;
                                return;
                            };
                        }
                        BubbledMessage::Piece(block) => {
                            // fail = Download closed its Receiver => Delegate not needed anymore
                            let Ok(_) = delegate.incoming_blocks.send(block).await else {
                                delegate.shutdown().await;
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
            let delegate = Arc::clone(&self);
            tokio::spawn(async move {
                on_shutdown.notified().await;
                delegate.ctx.add_active_connections(-1);
            });
        }

        let id = *connection.id();
        self.ctx.add_active_connections(1);
        self.connected_peers.lock().insert(id, connection);
    }
}
