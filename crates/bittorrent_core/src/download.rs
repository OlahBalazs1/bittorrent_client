use std::{fmt::Display, sync::Arc};

use rand::{RngExt, random, rng};
use tokio::sync::Mutex;

use crate::{
    announce,
    metainfo::Metainfo,
    network::{MinimalOpts, NetworkDelegate},
};

pub struct Download {
    network_delegate: Arc<Mutex<NetworkDelegate>>,
    pub data: DownloadData,
}

pub struct DownloadData {
    pub metainfo: Metainfo,
}

impl Download {
    pub(crate) async fn new(
        metainfo: Metainfo,
        network_delegate: Arc<Mutex<NetworkDelegate>>,
    ) -> Self {
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

        download
    }

    pub async fn announce(&mut self, opts: MinimalOpts) {
        self.network_delegate
            .lock()
            .await
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

fn generate_peer_id() -> String {
    let mut out = String::new();
    let mut rng = rng();
    for _ in 0..20 {
        out.push(match rng.random_range(0..3) {
            // lowercase alphabet
            0 => rng.random_range('a'..='z'),
            // uppercase alphabet
            1 => rng.random_range('A'..='B'),
            // number
            2 => rng.random_range('0'..='9'),
            _ => unreachable!(),
        });
    }
    out
}
