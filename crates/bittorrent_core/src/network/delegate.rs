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
        ExtendedOpts, MinimalOpts, NetworkContext,
        peer_connection::{BubbledMessage, InactivePeerConnection, PeerConnectionIo},
    },
    pieces::{Block, BlockRequest},
};

pub(crate) struct NetworkDelegateEvents {
    incoming_blocks: mpsc::Sender<Block>,
    piece_requests: mpsc::Receiver<BlockRequest>,
}

struct NetworkDelegateTasks {
    automatic_reannounce: Option<JoinHandle<()>>,
}

pub(crate) struct NetworkDelegate {
    ctx: Arc<TokioMutex<NetworkContext>>,
    info_hash: [u8; 20],
    peer_id: [u8; 20],

    last_announce: Mutex<Option<AnnounceResponse>>,

    block_recv: mpsc::Receiver<Block>,
    request_send: mpsc::Sender<BlockRequest>,

    tasks: nonpoison::Mutex<NetworkDelegateTasks>,

    // { peer_id: index_in_peer_connections}
    connected_peers: Mutex<HashMap<[u8; 20], Arc<PeerConnection>>>,
}

impl NetworkDelegate {
    pub(super) fn new(
        ctx: Arc<TokioMutex<NetworkContext>>,
        info_hash: [u8; 20],
        peer_id: [u8; 20],
    ) -> (Self, NetworkDelegateEvents) {
        let (block_send, block_recv) = mpsc::channel::<Block>(128);
        let (request_send, request_recv) = mpsc::channel::<BlockRequest>(128);

        let delegate = Self {
            ctx,
            last_announce: Mutex::new(None),
            connected_peers: Default::default(),
            info_hash,
            peer_id,
            block_recv,
            request_send,

            tasks: Mutex::new(NetworkDelegateTasks {
                automatic_reannounce: None,
            }),
        };

        (
            delegate,
            NetworkDelegateEvents {
                incoming_blocks: block_send,
                piece_requests: request_recv,
            },
        )
    }

    pub(crate) async fn announce(
        &mut self,
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
        *self.last_announce.lock() = Some(
            self.ctx
                .lock()
                .await
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
                .await?,
        );
        Ok(())
    }

    pub(crate) async fn shutdown(&self) {
        if let Some(reannounce) = self.tasks.lock().automatic_reannounce.take() {
            reannounce.abort();
        }
    }

    async fn start_automatic_reannounce_task(self: Arc<Self>) {
        todo!()
    }
    pub(crate) async fn register_connection(&mut self, connection: InactivePeerConnection) {
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
            on_shutdown: _,
        } = events;

        connection
            .send_handshake(&self.info_hash, &self.peer_id)
            .await;

        tokio::spawn(async move {
            while let Some(message) = bubble_recv.recv().await {
                match message {
                    BubbledMessage::Request(_request) => todo!(),
                    BubbledMessage::Piece(_piece) => todo!(),
                    BubbledMessage::Have(_have) => todo!(),
                    BubbledMessage::Bitfield(_bitfield) => todo!(),
                }
            }
        });

        let id = *connection.id();
        self.connected_peers.lock().insert(id, connection);
    }
}
