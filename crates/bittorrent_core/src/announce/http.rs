use bencode::parse_bencode;
use reqwest::ClientBuilder;

use crate::{
    announce::{
        Announce,
        AnnounceError::{BitTorrent, Network, Unknown},
        parse_peer_list,
    },
    session::SessionData,
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
    ) -> Result<Vec<super::Peer>, super::AnnounceError> {
        let url = format!(
            "{}?{}",
            session_data.metainfo.announce(),
            opts.to_uri_query_parameters()
        );

        let response = self.client.get(url).send().await.map_err(|e| Network(e))?;

        let response: String = response.text().await.map_err(|e| Network(e))?;

        let mut bdecoded = parse_bencode(response.as_bytes())
            .unwrap()
            .cast_dictionary()
            .unwrap();
        println!("{:#?}", bdecoded);

        if bdecoded.contains_key("error") {
            return Err(BitTorrent(
                bdecoded.remove("error").unwrap().cast_string().unwrap(),
            ));
        }

        // decoding the list just to reencode it again is a little wasteful, but this is the simplest way
        Ok(parse_peer_list(bdecoded.remove("peers").ok_or(Unknown)?.bencode()).ok_or(Unknown)?)
    }
}
