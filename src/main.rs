use bittorrent_core::{
    announce::http::HttpAnnouncer, download::FsOptions, metainfo::Metainfo, session::Session,
};
use log::info;

// test files:
// book.torrent: a book from internet archive, multifile with an UDP announce URL
// sword.torrent: an anime from nyaa.si, multifile
// youjo_senki.torrent: an anime from nyaa.si, single file

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter(None, log::LevelFilter::Info)
        .init();
    info!("env logger works");
    let mut session = Session::new(None, HttpAnnouncer::new()).await;

    let metainfo = Metainfo::from_metainfo_file(include_bytes!("youjo_senki.torrent"));
    session
        .add_download(
            metainfo,
            FsOptions {
                out_dir: ".".into(),
                preinitialize_file_length: true,
            },
        )
        .await;
}
