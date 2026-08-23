use reqwest::ClientBuilder;

use crate::announce::{Announce, AnnounceError::*, AnnounceResponse};

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
        announce_url: &str,
        opts: super::AnnounceOpts,
    ) -> Result<AnnounceResponse, super::AnnounceError> {
        let url = format!("{}?{}", announce_url, opts.to_uri_query_parameters());

        let response = self.client.get(url).send().await.map_err(|e| Network(e))?;

        let response = response.bytes().await.map_err(|e| Network(e))?;

        AnnounceResponse::from_bytes(&response)
    }
}
