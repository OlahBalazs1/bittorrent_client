use bencode::{Value, from_bytes, to_bytes};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

// TODO: Multitracker support (https://www.bittorrent.org/beps/bep_0012.html)
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Metainfo {
    announce: String,
    info_hash: [u8; 20],

    pieces: Vec<[u8; 20]>,
    piece_length: usize,
    wrapper_dir: Option<String>,
    files: Vec<FileDescriptor>,
}

impl Metainfo {
    pub fn from_metainfo_file(bencode: &[u8]) -> Self {
        #[derive(Deserialize)]
        struct InfoUnwrap {
            info: Value,
        }
        #[derive(Deserialize)]
        struct RawMetainfo<'a> {
            announce: String,

            #[serde(borrow)]
            info: InfoDict<'a>,
        }
        let parsed: RawMetainfo = from_bytes(bencode).unwrap();
        let RawMetainfo { announce, mut info } = parsed;
        let as_value: InfoUnwrap = from_bytes(bencode).unwrap();
        let info_hash: [u8; 20] = Sha1::digest(to_bytes(&as_value.info).unwrap()).into();
        let pieces = bytemuck::cast_slice::<u8, [u8; 20]>(info.pieces).to_vec();

        let (files, wrapper_dir) = match (info.single_file.take(), info.multi_file.take()) {
            // TODO: use Result
            (None, None) => panic!("Invalid metainfo!"),
            (Some(SingleFileInfo { name, length }), None) => (
                vec![FileDescriptor {
                    length,
                    path: vec![String::from_utf8_lossy(name).into_owned()],
                }],
                None,
            ),
            (None, Some(MultiFileInfo { name, files })) => {
                (files, Some(String::from_utf8_lossy(name).into_owned()))
            }
            // TODO: use Result
            (Some(_), Some(_)) => panic!("Invalid metainfo!"),
        };
        Self {
            announce,
            info_hash,
            pieces,
            piece_length: info.piece_length,
            wrapper_dir,
            files,
        }
    }

    pub fn announce(&self) -> &str {
        &self.announce
    }

    pub fn info_hash(&self) -> &[u8; 20] {
        &self.info_hash
    }
    pub fn pieces(&self) -> &[[u8; 20]] {
        &self.pieces
    }

    pub fn piece_length(&self) -> usize {
        self.piece_length
    }

    pub fn wrapper_dir(&self) -> Option<&str> {
        self.wrapper_dir.as_deref()
    }

    pub fn files(&self) -> &[FileDescriptor] {
        &self.files
    }
}

#[derive(Deserialize, Serialize)]
pub struct InfoDict<'a> {
    #[serde(rename = "piece length")]
    piece_length: usize,
    #[serde(with = "serde_bytes")]
    pieces: &'a [u8],

    #[serde(default)]
    #[serde(flatten)]
    single_file: Option<SingleFileInfo<'a>>,

    #[serde(default)]
    #[serde(flatten)]
    multi_file: Option<MultiFileInfo<'a>>,
}
#[derive(Deserialize, Serialize)]
struct SingleFileInfo<'a> {
    #[serde(with = "serde_bytes")]
    name: &'a [u8],
    length: usize,
}

#[derive(Deserialize, Serialize)]
struct MultiFileInfo<'a> {
    #[serde(with = "serde_bytes")]
    name: &'a [u8],
    files: Vec<FileDescriptor>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FileDescriptor {
    length: usize,
    path: Vec<String>,
}

impl FileDescriptor {
    pub fn length(&self) -> usize {
        self.length
    }
    pub fn path(&self) -> &[String] {
        &self.path
    }
}
