use bittorrent_core::{announce::http::HttpAnnouncer, metainfo::Metainfo, session::SessionBuilder};

#[tokio::main]
async fn main() {
    let metainfo = Metainfo::parse(include_bytes!("youjo_senki.torrent"));
    let mut session = SessionBuilder::default().build(metainfo, HttpAnnouncer::new());

    let peerlist = session.announce().await;
    println!("{:#?}", peerlist);
}
