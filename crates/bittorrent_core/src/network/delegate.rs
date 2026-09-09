use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use log::info;
use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream},
    sync::Mutex,
};

use super::peer_connection::{Peer, PeerConnection};

use crate::{
    announce::{Announce, AnnounceError, AnnounceEvent, AnnounceOpts, AnnounceResponse},
    network::{
        ExtendedOpts, MinimalOpts, NetworkContext, Queue, message::Message,
        peer_connection::InactivePeerConnection,
    },
};
pub(crate) struct NetworkDelegate {
    ctx: Arc<Mutex<NetworkContext>>,
    info_hash: [u8; 20],
    peer_id: [u8; 20],

    raw_peers: Vec<Peer>,

    last_announce: Option<AnnounceResponse>,

    incoming_buffer: Arc<Mutex<Queue<Message>>>,

    // { peer_id: index_in_peer_connections}
    connected_peers: HashMap<[u8; 20], Arc<Mutex<PeerConnection>>>,
}

impl NetworkDelegate {
    pub(super) fn new(
        ctx: Arc<Mutex<NetworkContext>>,
        info_hash: [u8; 20],
        peer_id: [u8; 20],
    ) -> Self {
        Self {
            ctx,
            raw_peers: Vec::new(),
            incoming_buffer: Default::default(),
            last_announce: None,
            connected_peers: HashMap::new(),
            info_hash,
            peer_id,
        }
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
        println!("{:#?}", self.last_announce);
        Ok(())
    }
    pub(crate) async fn register_connection(&mut self, connection: InactivePeerConnection) {
        if self.connected_peers.contains_key(connection.id() as &[u8]) {
            return;
        }
        let (connection, msg_bubble) = connection.activate(Arc::clone(&self.incoming_buffer)).await;

        connection
            .lock()
            .await
            .send_handshake(&self.info_hash, &self.peer_id)
            .await;

        let id = *connection.lock().await.id();
        self.connected_peers.insert(id, connection);
    }
}
