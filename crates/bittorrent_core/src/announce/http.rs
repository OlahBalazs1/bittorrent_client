use reqwest::ClientBuilder;

use crate::{announce::Announce, session::SessionData};

pub struct HttpAnnouncer {
    client: reqwest::Client,
}

impl HttpAnnouncer {
    pub fn new() -> Self {
        let client = ClientBuilder::new()
            .user_agent("restaurrent")
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
        opts: super::AnnounceEvent,
    ) -> Result<Vec<super::Peer>, super::AnnounceError> {
        todo!();
    }
}
