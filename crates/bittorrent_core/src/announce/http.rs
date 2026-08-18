use bencode::parse_bencode;
use reqwest::ClientBuilder;

use crate::announce::{
    Announce,
    AnnounceError::{self, BitTorrent, Network, Unknown},
    AnnounceResponse, parse_peer_list,
};

pub struct HttpAnnouncer {
    client: reqwest::Client,
}

impl HttpAnnouncer {
    pub fn new() -> Self {
        let client = ClientBuilder::new()
            .user_agent("placeholder_name")
            .build()
            .unwrap();

        Self { client }
    }
}

#[async_trait::async_trait]
impl Announce for HttpAnnouncer {
    async fn announce(
        &mut self,
        session_data: &crate::session::SessionData,
        opts: super::AnnounceOpts,
    ) -> Result<AnnounceResponse, super::AnnounceError> {
        let url = format!(
            "{}?{}",
            session_data.metainfo.announce(),
            opts.to_uri_query_parameters()
        );

        let response = self.client.get(url).send().await.map_err(|e| Network(e))?;

        let response = response.bytes().await.map_err(|e| Network(e))?;
        let bdecoded = parse_bencode(&response).map_err(|_| Unknown)?;

        AnnounceResponse::parse_bdecoded(bdecoded)
    }
}
