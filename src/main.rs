use bittorrent_core::{
    announce::http::HttpAnnouncer, download::FsOptions, metainfo::Metainfo, session::Session,
};
use log::info;

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter(None, log::LevelFilter::Info)
        .init();
    info!("env logger works");
    let mut session = Session::new(None, HttpAnnouncer::new()).await;

    let metainfo = Metainfo::from_metainfo_file(include_bytes!("archlinux.torrent"));
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
