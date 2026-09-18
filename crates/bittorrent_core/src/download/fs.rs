use std::{
    ops::Deref,
    path::PathBuf,
    sync::{Arc, nonpoison},
};

use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncReadExt, AsyncSeekExt, AsyncWrite, AsyncWriteExt},
    sync::{
        Mutex as TokioMutex,
        mpsc::{self, Receiver},
    },
    task::JoinSet,
};
use winnow::stream::Range;

use crate::{
    metainfo::{self, Metainfo},
    pieces::{Block, BlockRequest},
};

#[derive(Clone)]
pub struct FsOptions {
    pub out_dir: PathBuf,
    pub preinitialize_file_length: bool,
}

pub(super) struct FsHandlerEvents {
    pub piece_completed: mpsc::Receiver<usize>,
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
        options: FsOptions,
    ) -> (Arc<Self>, FsHandlerEvents) {
        let FsOptions {
            mut out_dir,
            preinitialize_file_length,
        } = options;

        // TODO wire it up
        let (piece_send, piece_recv) = mpsc::channel::<usize>(128);

        if let Some(wrapper) = metainfo.wrapper_dir() {
            out_dir.push(wrapper);
        }

        let mut file_opts = OpenOptions::new();
        file_opts.write(true).read(true).create(true);

        let mut files = Vec::with_capacity(metainfo.files().len());

        for descriptor in metainfo.files() {
            let mut path = out_dir.clone();
            for stuff in descriptor.path() {
                path.push(stuff);
            }

            let file_opts = file_opts.clone();
            let file = file_opts.open(path).await.unwrap();
            if preinitialize_file_length {
                file.set_len(descriptor.length() as _).await.unwrap();
            }
            let file_handle = Arc::new(FileHandle {
                file: TokioMutex::new(file),
                length: descriptor.length(),
            });
            files.push(file_handle);
        }

        (
            Arc::new(Self {
                files,
                piece_length: metainfo.piece_length(),
                piece_hashes: metainfo.pieces().to_vec(),
            }),
            FsHandlerEvents {
                piece_completed: piece_recv,
            },
        )
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
        let piece_end = if piece == self.piece_hashes.len() - 1 {
            piece * self.piece_length + self.files.last().unwrap().length
        } else {
            (piece + 1) * self.piece_length
        };
        let piece = (piece * self.piece_length)..(piece_end);
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
