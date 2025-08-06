#[cfg_attr(target_os = "linux", path = "inotify.rs")]
mod backend;

pub use backend::*;

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::task::{Context, Poll};

use pin_project_lite::pin_project;
use tokio::io::{AsyncRead, ReadBuf};

impl Registration {
    pub fn watch(&self, path: &Path, offset: u64) -> std::io::Result<FileReader> {
        let mut file = OpenOptions::new().read(true).open(path)?;
        if offset != 0 {
            file.seek(SeekFrom::Start(offset))?;
        }

        let handle = self.add(path)?;

        Ok(FileReader {
            file,
            path: path.to_path_buf(),
            handle,
        })
    }
}

pin_project! {
    pub struct FileReader {
        #[pin]
        file: File,

        path: PathBuf,
        handle: Handle
    }
}

impl AsyncRead for FileReader {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let mut this = self.project();

        let unfilled = buf.initialize_unfilled();
        match this.file.read(unfilled) {
            Ok(size) => {
                if size == 0 {
                    this.handle.register(cx.waker());
                    return Poll::Pending;
                }

                buf.advance(size);
                Poll::Ready(Ok(()))
            }
            Err(err) => Poll::Ready(Err(err)),
        }
    }
}

impl FileReader {
    pub fn new(file: File, path: PathBuf, handle: Handle) -> Self {
        Self { file, path, handle }
    }

    #[inline]
    pub fn path(&self) -> &Path {
        &self.path
    }
}
