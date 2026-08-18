use bencode::parse_bencode;
use reqwest::ClientBuilder;

use crate::announce::{
        Announce,
        AnnounceError::{BitTorrent, Network, Unknown},
        parse_peer_list,
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

        let response = response.bytes().await.map_err(|e| Network(e))?;

        let mut bdecoded = parse_bencode(&response).unwrap().cast_dictionary().unwrap();
        println!("{:#?}", bdecoded);

        if bdecoded.contains_key("error") {
            return Err(BitTorrent(
                bdecoded.remove("error").unwrap().cast_string().unwrap(),
            ));
        }

        Ok(parse_peer_list(bdecoded.remove("peers").ok_or(Unknown)?).ok_or(Unknown)?)
    }
}
