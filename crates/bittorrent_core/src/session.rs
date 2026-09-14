use std::sync::Arc;

use tokio::sync::Mutex as TokioMutex;

use crate::{
    announce::Announce,
    download::Download,
    metainfo::Metainfo,
    network::{NetworkContext, create_tcp_listener},
    util::generate_peer_id,
};

pub struct Session {
    downloads: Vec<Download>,
    network_ctx: Arc<NetworkContext>,
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

        let network_ctx = Arc::new(network_ctx);

        Arc::clone(&network_ctx).start_listener(listener);

        Self {
            network_ctx,
            downloads: Vec::new(),
        }
    }

    pub async fn add_download(&mut self, metainfo: Metainfo) {
        let Some(download) = Download::new(metainfo, Arc::clone(&self.network_ctx)).await else {
            return;
        };
        self.downloads.push(download);
    }
}
