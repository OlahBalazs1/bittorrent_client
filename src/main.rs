use bittorrent_core::{announce::http::HttpAnnouncer, metainfo::Metainfo, session::Session};

#[tokio::main]
async fn main() {
    let mut session = Session::new(None, HttpAnnouncer::new()).await;

    let metainfo = Metainfo::parse(include_bytes!("youjo_senki.torrent"));
    session.add_download(metainfo).await;
}
