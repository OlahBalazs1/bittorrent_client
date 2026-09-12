use std::{fmt::Display, sync::Arc};

use tokio::sync::Mutex;

mod fs;

use crate::{
    announce,
    metainfo::Metainfo,
    network::{MinimalOpts, NetworkContext, delegate::NetworkDelegate},
    util::generate_peer_id,
};

pub struct Download {
    network_delegate: Arc<NetworkDelegate>,
    pub data: DownloadData,
}

pub struct DownloadData {
    pub metainfo: Metainfo,
}

impl Download {
    pub(crate) async fn new(metainfo: Metainfo, network_ctx: Arc<NetworkContext>) -> Option<Self> {
        let peer_id = generate_peer_id();
        let network_delegate = network_ctx
            .add_delegate(metainfo.info_hash, peer_id)
            .await?;
        let mut download = Self {
            network_delegate,
            data: DownloadData { metainfo },
        };

        download
            .announce(MinimalOpts {
                uploaded: 0,
                downloaded: 0,
                left: 0,
                event: announce::AnnounceEvent::Started,
                compact: false,
            })
            .await;

        Some(download)
    }

    pub async fn announce(&mut self, opts: MinimalOpts) {
        self.network_delegate
            .announce(&self.data.metainfo.announce, opts)
            .await
            .unwrap();
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum Protocol {
    #[default]
    TCP,
    UDP,
}

impl Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Protocol::TCP => write!(f, "TCP"),
            Protocol::UDP => write!(f, "UDP"),
        }
    }
}
