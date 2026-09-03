use bittorrent_core::{announce::http::HttpAnnouncer, metainfo::Metainfo, session::Session};
use log::info;

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter(None, log::LevelFilter::Info)
        .init();
    info!("env logger works");
    let mut session = Session::new(None, HttpAnnouncer::new()).await;

    let metainfo = Metainfo::parse(include_bytes!("youjo_senki.torrent"));
    session.add_download(metainfo).await;
}
