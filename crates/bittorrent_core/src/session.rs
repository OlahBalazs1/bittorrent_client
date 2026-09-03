use std::sync::Arc;

use rand::random;
use tokio::sync::Mutex;

use crate::{
    announce::Announce,
    download::Download,
    metainfo::Metainfo,
    network::{NetworkContext, create_tcp_listener, start_listener},
    util::generate_peer_id,
};

pub struct Session {
    downloads: Vec<Download>,
    network_ctx: Arc<Mutex<NetworkContext>>,
}

impl Session {
    pub async fn new<A: Announce + Send + Sync + 'static>(
        listener_port: Option<u16>,
        announcer: A,
    ) -> Self {
        let (listener, port) = create_tcp_listener(listener_port).await.unwrap();
        let Some(network_ctx) = NetworkContext::new(port, announcer).await else {
            panic!("Could not create session");
        };

        let network_ctx = Arc::new(Mutex::new(network_ctx));

        start_listener(Arc::clone(&network_ctx), listener);

        Self {
            network_ctx,
            downloads: Vec::new(),
        }
    }

    pub async fn add_download(&mut self, metainfo: Metainfo) {
        let peer_id = generate_peer_id();
        let Some(delegate) = NetworkContext::add_delegate(
            Arc::clone(&self.network_ctx),
            metainfo.info_hash,
            peer_id,
        )
        .await
        else {
            return;
        };
        let download = Download::new(metainfo, delegate).await;
        self.downloads.push(download);
    }
}
