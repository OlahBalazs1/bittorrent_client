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
        let RawMetainfo { announce, info } = parsed;
        let single_file: Option<SingleFileInfo> = from_bytes::<SingleFileInfoDict>(bencode)
            .ok()
            .map(|e| e.info);
        let multi_file: Option<MultiFileInfo> = from_bytes::<MultiFileInfoDict>(bencode)
            .ok()
            .map(|e| e.info);
        let as_value: InfoUnwrap = from_bytes(bencode).unwrap();
        let info_hash: [u8; 20] = Sha1::digest(to_bytes(&as_value.info).unwrap()).into();
        let pieces = bytemuck::cast_slice::<u8, [u8; 20]>(info.pieces).to_vec();

        let (files, wrapper_dir) = match (single_file, multi_file) {
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
}

#[derive(Deserialize, Serialize, PartialEq, Debug)]
pub struct MultiFileInfoDict<'a> {
    #[serde(borrow)]
    info: MultiFileInfo<'a>,
}
#[derive(Deserialize, Serialize, PartialEq, Debug)]
pub struct SingleFileInfoDict<'a> {
    #[serde(borrow)]
    info: SingleFileInfo<'a>,
}
#[derive(Deserialize, Serialize, PartialEq, Debug)]
struct SingleFileInfo<'a> {
    #[serde(with = "serde_bytes")]
    name: &'a [u8],
    length: usize,
}

#[derive(Deserialize, Serialize, PartialEq, Debug)]
struct MultiFileInfo<'a> {
    #[serde(with = "serde_bytes")]
    name: &'a [u8],
    files: Vec<FileDescriptor>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde_bytes::ByteBuf;

    use super::*;

    #[test]
    fn file_list() {
        let test_case = "d
        4:info
            d
                4:name4:name
                5:files
                l
                    d
                    6:lengthi32e
                    4:path
                        l
                            4:dir1
                            4:dir2
                            8:file.ext
                        e
                    e
                e
            e
        e";

        let test_case = test_case
            .chars()
            .filter(|e| !e.is_whitespace())
            .collect::<String>();
        let single_file_info: Option<SingleFileInfoDict> = None;
        let multi_file_info = Some(MultiFileInfoDict {
            info: MultiFileInfo {
                name: "name".as_bytes(),
                files: vec![FileDescriptor {
                    length: 32,
                    path: vec!["dir1".into(), "dir2".into(), "file.ext".into()],
                }],
            },
        });
        let parsed_none: Option<SingleFileInfoDict> =
            bencode::from_bytes(test_case.as_bytes()).ok();
        let parsed_some: Option<MultiFileInfoDict> = bencode::from_bytes(test_case.as_bytes()).ok();
        // panic!("{:?}", parsed);
        pretty_assertions::assert_eq!(parsed_none, single_file_info);
        pretty_assertions::assert_eq!(parsed_some, multi_file_info);
    }
}
