use std::{
    ops::Deref,
    sync::{Arc, nonpoison},
};

use tokio::{
    fs::File,
    io::{AsyncSeekExt, AsyncWrite, AsyncWriteExt},
    sync::{Mutex as TokioMutex, mpsc::Receiver},
};
use winnow::stream::Range;

use crate::{
    metainfo::Metainfo,
    pieces::{Block, BlockRequest},
};

pub struct FsOptions {}

pub(super) struct FsHandlerEvents {
    piece_completed: Receiver<usize>,
}

struct FileHandle {
    file: TokioMutex<File>,
    length: usize,
}

struct Piece<'a> {
    files: &'a [Arc<FileHandle>],
    start_offset: usize,
    end_truncation: usize,
}

pub(super) struct FsHandler {
    files: Vec<Arc<FileHandle>>,
}

impl FsHandler {
    pub(crate) async fn new(
        metainfo: &Metainfo,
        fs_options: FsOptions,
    ) -> (Arc<Self>, FsHandlerEvents) {
        todo!()
    }
    pub(crate) async fn write_block(&self, block: Block) {
        let Block { piece, begin, data } = block;
        self.get_piece(piece).await.write(begin, data.into()).await
    }

    pub(crate) async fn get_piece(&self, piece: usize) -> Piece<'_> {
        todo!()
    }

    pub(crate) async fn handle_request(&self, request: BlockRequest) {
        todo!()
    }
}

impl<'a> Piece<'a> {
    async fn write(&self, begin: usize, data: Arc<[u8]>) {
        let mut written = 0;
        let mut cursor = 0;

        for (index, file) in self.files.iter().enumerate() {
            if written > data.len() {
                break;
            } else if written == data.len() {
                panic!(
                    "Piece::write() is royally fucked, as it wrote more than the length of the data it was given."
                )
            }
            let mut length = file.length;
            if index == 0 {
                length -= self.start_offset;
            }
            if index == self.files.len() - 1 {
                length -= self.end_truncation;
            }
            if cursor < begin {
                if begin - cursor > length {
                    // in file so set it to be at begin
                    cursor += begin - cursor
                } else {
                    // the begin is not in this file, so continue
                    cursor += length;
                    continue;
                };
            }

            let to_write = std::cmp::min(data.len() - written, length);
            {
                let data = Arc::clone(&data);
                let start_offset = self.start_offset;
                let file = Arc::clone(&file);
                tokio::spawn(async move {
                    let mut file = file.lock().await;
                    if index == 0 {
                        file.seek(std::io::SeekFrom::Start(start_offset as _))
                            .await
                            .unwrap();
                    }
                    file.write_all(&data[written..(written + to_write)])
                        .await
                        .unwrap();
                });
            }
            written += to_write;
        }

        if written < data.len() {
            // TODO: return an error stating that the write failed
            todo!()
        }
    }

    async fn read(&self, begin: usize, buf: &mut [u8]) {
        let mut written = 0;
        let mut cursor = 0;

        let mut files = Vec::new();
        let mut ranges = Vec::new();
        for (index, file) in self.files.iter().enumerate() {
            if written > buf.len() {
                break;
            } else if written == buf.len() {
                panic!(
                    "Piece::write() is royally fucked, as it wrote more than the length of the data it was given."
                )
            }
            let mut length = file.length;
            if index == 0 {
                length -= self.start_offset;
            }
            if index == self.files.len() - 1 {
                length -= self.end_truncation;
            }
            if cursor < begin {
                if begin - cursor > length {
                    // in file so set it to be at begin
                    cursor += begin - cursor
                } else {
                    // the begin is not in this file, so continue
                    cursor += length;
                    continue;
                };
            }

            let to_write = std::cmp::min(buf.len() - written, length);

            files.push(Arc::clone(file));
            ranges.push(written..(to_write + written));
            written += to_write;
        }

        if written < buf.len() {
            // TODO: return an error stating that the write failed
            todo!()
        }
        let mutex = nonpoison::Mutex::new(buf);

        todo!();
    }
}

impl Deref for FileHandle {
    type Target = TokioMutex<File>;
    fn deref(&self) -> &Self::Target {
        &self.file
    }
}
