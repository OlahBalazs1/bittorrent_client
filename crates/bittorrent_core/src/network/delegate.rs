use std::{collections::HashMap, sync::Arc};

use log::info;
use tokio::sync::{
    Mutex,
    mpsc::{self, Receiver},
    oneshot::channel,
    watch::Sender,
};

use super::peer_connection::{Peer, PeerConnection};

use crate::{
    announce::{AnnounceError, AnnounceResponse},
    network::{
        ExtendedOpts, MinimalOpts, NetworkContext, Queue,
        message::Message,
        peer_connection::{InactivePeerConnection, PeerConnectionIo},
    },
    pieces::{Block, PieceRequest},
};

pub(crate) struct NetworkDelegateEvents {
    incoming_blocks: Receiver<Block>,
    piece_requests: Receiver<PieceRequest>,
}

pub(crate) struct NetworkDelegate {
    ctx: Arc<Mutex<NetworkContext>>,
    info_hash: [u8; 20],
    peer_id: [u8; 20],

    raw_peers: Vec<Peer>,

    last_announce: Option<AnnounceResponse>,

    block_send: mpsc::Sender<Block>,
    request_send: mpsc::Sender<PieceRequest>,

    // { peer_id: index_in_peer_connections}
    connected_peers: Arc<Mutex<HashMap<[u8; 20], Arc<Mutex<PeerConnection>>>>>,
}

impl NetworkDelegate {
    pub(super) fn new(
        ctx: Arc<Mutex<NetworkContext>>,
        info_hash: [u8; 20],
        peer_id: [u8; 20],
    ) -> (Self, NetworkDelegateEvents) {
        let (block_send, block_recv) = mpsc::channel::<Block>(128);
        let (request_send, request_recv) = mpsc::channel::<PieceRequest>(128);

        let delegate = Self {
            ctx,
            raw_peers: Vec::new(),
            last_announce: None,
            connected_peers: Default::default(),
            info_hash,
            peer_id,
            block_send,
            request_send,
        };

        (
            delegate,
            NetworkDelegateEvents {
                incoming_blocks: block_recv,
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
        self.last_announce = Some(
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
    pub(crate) async fn register_connection(&mut self, connection: InactivePeerConnection) {
        if self
            .connected_peers
            .lock()
            .await
            .contains_key(connection.id() as &[u8])
        {
            return;
        }
        let (connection, events) = connection.activate().await;
        let PeerConnectionIo {
            mut bubble_recv,
            shutdown,
        } = events;
        let mut block_send = self.block_send.clone();
        let mut request_send = self.request_send.clone();

        tokio::spawn(async move {
            while let Some(message) = bubble_recv.recv().await {
                match message {
                    Message::Cancel {
                        index,
                        begin,
                        length,
                    } => todo!(),
                    Message::Piece {
                        index,
                        begin,
                        block,
                    } => {
                        let Ok(_) = block_send
                            .send(Block {
                                piece: index,
                                begin,
                                data: block,
                            })
                            .await
                        else {
                            break;
                        };
                    }
                    Message::Request {
                        index,
                        begin,
                        length,
                    } => {
                        let request = PieceRequest::new(index, begin, length, return_path);
                        request_send.send(request).await;
                    }

                    // supposed to be handled by the PeerConnection
                    Message::Choke
                    | Message::Interested
                    | Message::KeepAlive
                    | Message::NotInterested
                    | Message::Unchoke
                    | Message::Bitfield(_)
                    | Message::Have(_) => {}
                }
            }
        });

        connection
            .lock()
            .await
            .send_handshake(&self.info_hash, &self.peer_id)
            .await;

        let id = *connection.lock().await.id();
        self.connected_peers.lock().await.insert(id, connection);
    }
}
