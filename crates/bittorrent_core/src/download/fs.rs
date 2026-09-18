use std::{
    ops::Deref,
    sync::{Arc, nonpoison},
};

use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncSeekExt, AsyncWrite, AsyncWriteExt},
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
    piece_length: usize,
    piece_hashes: Vec<[u8; 20]>,
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
        self.get_piece(piece).write(begin, data.as_ref()).await
    }

    // O(n) where n = files.len()
    fn get_piece(&self, piece: usize) -> Piece<'_> {
        let mut first_file: usize = 0;
        let mut last_file: usize = 0;
        let mut inside_piece = false;
        let piece = (piece * self.piece_length)..(piece + 1 * self.piece_length);
        let mut start_offset = 0;
        let mut end_truncation = 0;

        let mut cumulative_length = 0;
        for (index, file) in self.files.iter().enumerate() {
            let start_of_file = cumulative_length;
            let end_of_file = start_of_file + file.length;
            if !inside_piece && piece.contains(&end_of_file) {
                first_file = index;
                inside_piece = true;
                start_offset = start_of_file - piece.start;
            }
            if inside_piece && piece.contains(&start_of_file) {
                last_file = index;
                end_truncation = piece.end - end_of_file;
                break;
            }

            cumulative_length += file.length;
        }

        Piece {
            files: &self.files[first_file..=last_file],
            start_offset,
            end_truncation,
        }
    }

    pub(crate) async fn handle_request(&self, mut request: BlockRequest) {
        let piece = self.get_piece(request.piece());
        let mut buf = vec![0u8; request.length()];

        piece.read(request.begin(), &mut buf).await;

        request.fulfill(buf.into_boxed_slice()).unwrap()
    }
}

impl<'a> Piece<'a> {
    async fn write(&self, begin: usize, mut data: &[u8]) {
        let mut written = 0;
        let mut cursor = 0;

        tokio_scoped::scope(|s| {
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
                let (left, right) = data.split_at(to_write);
                data = right;
                {
                    let start_offset = self.start_offset;
                    let file = Arc::clone(&file);
                    s.spawn(async move {
                        let mut file = file.lock().await;
                        if index == 0 {
                            file.seek(std::io::SeekFrom::Start(start_offset as _))
                                .await
                                .unwrap();
                        }
                        file.write_all(left).await.unwrap();
                    });
                }
                written += to_write;
            }
        });

        if written < data.len() {
            // TODO: return an error stating that the write failed
            todo!()
        }
    }

    async fn read(&self, begin: usize, mut buf: &mut [u8]) {
        let mut written = 0;
        let mut cursor = 0;

        tokio_scoped::scope(|s| {
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
                let (left, right) = buf.split_at_mut(to_write);
                let start_offset = self.start_offset;
                buf = right;
                s.spawn(async move {
                    let mut file = file.lock().await;
                    if index == 0 {
                        file.seek(std::io::SeekFrom::Start(start_offset as _))
                            .await
                            .unwrap();
                    }
                    let written = file.read_exact(left).await.unwrap();
                    if written != to_write {
                        panic!("Fuck")
                    }
                });

                written += to_write;
            }
        });
    }
}

impl Deref for FileHandle {
    type Target = TokioMutex<File>;
    fn deref(&self) -> &Self::Target {
        &self.file
    }
}
