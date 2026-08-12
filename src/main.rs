use bittorrent_core::metainfo::Metainfo;

fn main() {
    let info = Metainfo::parse(include_bytes!("youjo_senki.torrent"));
    println!("{:?}", info);
}
