use std::{
    fmt::Display,
    sync::{Arc, nonpoison},
};

use log::info;
use tokio::{sync::Mutex, task::JoinSet};

mod fs;

pub use fs::FsOptions;

use crate::{
    announce,
    download::fs::FsHandler,
    metainfo::Metainfo,
    network::{
        MinimalOpts, NetworkContext,
        delegate::{NetworkDelegate, NetworkDelegateIo},
    },
    util::generate_peer_id,
};

pub struct Download {
    network_delegate: Arc<NetworkDelegate>,
    pub data: DownloadData,
    fs: Arc<FsHandler>,
    tasks: nonpoison::Mutex<JoinSet<()>>,
}

pub struct DownloadData {
    pub metainfo: Metainfo,
}

impl Download {
    pub(crate) async fn new(
        metainfo: Metainfo,
        network_ctx: Arc<NetworkContext>,
        fs_options: FsOptions,
    ) -> Option<Arc<Self>> {
        let peer_id = generate_peer_id();
        let (network_delegate, delegate_io) = network_ctx
            .add_delegate(metainfo.info_hash, peer_id)
            .await?;

        let (fs_handler, fs_events) = FsHandler::new(&metainfo, fs_options).await;

        let NetworkDelegateIo {
            mut piece_requests,
            mut incoming_blocks,
        } = delegate_io;

        let download = Arc::new(Self {
            network_delegate,
            data: DownloadData { metainfo },
            fs: fs_handler,
            tasks: nonpoison::Mutex::new(JoinSet::new()),
        });

        // initial announce
        let initial_announce = {
            let down = Arc::clone(&download);
            tokio::spawn(async move {
                down.announce(MinimalOpts {
                    uploaded: 0,
                    downloaded: 0,
                    left: 0,
                    event: announce::AnnounceEvent::Started,
                    compact: false,
                })
                .await
            })
        };
        // piece_requests
        {
            let down = Arc::clone(&download);
            download.tasks.lock().spawn(async move {
                while let Some(block_request) = piece_requests.recv().await {
                    info!(
                        "Block request received for piece: {}, begin: {}, len: {} bytes",
                        block_request.piece(),
                        block_request.begin(),
                        block_request.length()
                    );

                    down.fs.handle_request(block_request).await;
                }
            });
        }
        // blocks
        {
            let down = Arc::clone(&download);
            download.tasks.lock().spawn(async move {
                while let Some(block) = incoming_blocks.recv().await {
                    down.fs.write_block(block).await;
                }
            });
        }

        initial_announce.await.unwrap();
        Some(download)
    }

    pub async fn announce(&self, opts: MinimalOpts) {
        self.network_delegate
            .announce(&self.data.metainfo.announce, opts)
            .await
            .unwrap();
    }

    pub async fn shutdown(&self) {
        info!("Stopping download...");
        self.tasks.lock().abort_all();
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
