#[cfg(test)]
mod tests;

use std::fs;
use std::io::{self, BufRead, BufReader, Seek};
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::time::Instant;

use bytes::{Bytes, BytesMut};
use chrono::{DateTime, Utc};
use flate2::bufread::MultiGzDecoder;
use tracing::debug;

use crate::buffer::read_until_with_max_size;
use crate::{Position, ReadFrom};

/// The `Watcher` struct defines the polling based state machine which reads from a file
/// path, transparently updating the underlying file descriptor when the file has been rolled
/// over, as is common for logs
///
/// The `Watcher` is expected to live for the lifetime of the file path. `Server` is
/// responsible for clearing away `Watchers` which no longer exist
pub struct Watcher {
    pub(crate) path: PathBuf,

    findable: bool,
    reader: Box<dyn BufRead>,
    position: Position,
    devno: u64,
    inode: u64,
    dead: bool,
    last_read_attempt: Instant,
    last_read_success: Instant,
    max_line_bytes: usize,
    line_delimiter: Bytes,
    buf: BytesMut,
}

impl Watcher {
    /// Create a new `Watcher`
    ///
    /// The input path will be used by `Watcher` to prime its state machine.
    /// A `Watcher` tracks _only one_ file. This function returns None if the path
    /// does not exist or is not readable by the current process.
    pub fn new(
        path: PathBuf,
        read_from: ReadFrom,
        ignore_before: Option<DateTime<Utc>>,
        max_line_bytes: usize,
        line_delimiter: Bytes,
    ) -> Result<Self, std::io::Error> {
        let f = std::fs::File::open(&path)?;
        let metadata = f.metadata()?;
        let devno = metadata.dev();
        let inode = metadata.ino();
        let mut reader = BufReader::new(f);

        let too_old = if let (Some(ignore_before), Ok(modified_time)) = (
            ignore_before,
            metadata.modified().map(DateTime::<Utc>::from),
        ) {
            modified_time < ignore_before
        } else {
            false
        };

        let gzipped = is_gzipped(&mut reader)?;

        // Determine the actual position at which we should start reading
        let (reader, position): (Box<dyn BufRead>, Position) = match (gzipped, too_old, read_from) {
            (true, true, _) => {
                debug!(message = "Not reading gzipped file older than `ignore_older`");

                (Box::new(null_reader()), 0)
            }
            (true, _, ReadFrom::Checkpoint(position)) => {
                debug!(
                    message = "Not re-reading gzipped frile with existing stored offset",
                    ?path,
                    %position
                );

                (Box::new(null_reader()), position)
            }
            // TODO: This may become the default, leading us to stop reading gzipped file that
            // we were reading before. Should we merge this and the next branch to read compressed
            // file from the beginning even when `read_from = "end"` (implicitly via default or
            // explicitly via config)?
            (true, _, ReadFrom::End) => {
                debug!(
                    message = "Can't read from the end of already-compressed file",
                    ?path
                );
                (Box::new(null_reader()), 0)
            }
            (true, false, ReadFrom::Beginning) => {
                (Box::new(io::BufReader::new(MultiGzDecoder::new(reader))), 0)
            }
            (false, true, _) => {
                let pos = reader.seek(io::SeekFrom::End(0)).unwrap();
                (Box::new(reader), pos)
            }
            (false, false, ReadFrom::Checkpoint(position)) => {
                let pos = reader.seek(io::SeekFrom::Start(position)).unwrap();
                (Box::new(reader), pos)
            }
            (false, false, ReadFrom::Beginning) => {
                let pos = reader.seek(io::SeekFrom::Start(0)).unwrap();
                (Box::new(reader), pos)
            }
            (false, false, ReadFrom::End) => {
                let pos = reader.seek(io::SeekFrom::End(0)).unwrap();
                (Box::new(reader), pos)
            }
        };

        let ts = metadata
            .modified()
            .ok()
            .and_then(|mtime| mtime.elapsed().ok())
            .and_then(|diff| Instant::now().checked_sub(diff))
            .unwrap_or_else(Instant::now);

        Ok(Self {
            path,
            findable: false,
            reader,
            position,
            devno,
            inode,
            dead: false,
            last_read_attempt: ts,
            last_read_success: ts,
            max_line_bytes,
            line_delimiter,
            buf: BytesMut::new(),
        })
    }

    #[inline]
    pub fn should_read(&self) -> bool {
        // TODO: implement this
        true
    }

    /// Read a single line from the underlying file
    ///
    /// This function will attempt to read a new line from its file, blocking up to some
    /// maximum but unspecified amount of time. `read_line` will open a new file handler
    /// as needed, transparently to the caller
    pub fn read_line(&mut self) -> io::Result<Option<Bytes>> {
        let reader = &mut self.reader;
        let pos = &mut self.position;

        match read_until_with_max_size(
            reader,
            pos,
            self.line_delimiter.as_ref(),
            &mut self.buf,
            self.max_line_bytes,
        ) {
            Ok(Some(_)) => Ok(Some(self.buf.split().freeze())),
            Ok(None) => {
                if !self.findable {
                    self.dead = true;
                    // File has been deleted, so return what we have in the buffer, even though it
                    // didn't end with a newline. This is not a perfect signal for when we should
                    // give up waiting for a newline, but it's decent.
                    let buf = self.buf.split().freeze();
                    if buf.is_empty() {
                        // EOF
                        Ok(None)
                    } else {
                        Ok(Some(buf))
                    }
                } else {
                    Ok(None)
                }
            }
            Err(err) => {
                if let io::ErrorKind::NotFound = err.kind() {
                    self.dead = true;
                }

                Err(err)
            }
        }
    }

    #[inline]
    pub fn file_position(&self) -> Position {
        self.position
    }

    pub fn file_findable(&self) -> bool {
        self.findable
    }

    fn track_read_attempt(&mut self) {
        self.last_read_attempt = Instant::now();
    }

    fn track_read_success(&mut self) {
        self.last_read_success = Instant::now();
    }

    pub fn set_findable(&mut self, b: bool) {
        self.findable = b;
    }

    pub fn dead(&self) -> bool {
        self.dead
    }
}

fn is_gzipped(r: &mut io::BufReader<fs::File>) -> io::Result<bool> {
    let header_bytes = r.fill_buf()?;
    // WARN: The paired `BufReader::consume` is not called intentionally. If we
    // do we'll chop a decent part of the potential gzip stream off.
    Ok(header_bytes.starts_with(&[0x1f, 0x8b]))
}

fn null_reader() -> impl BufRead {
    io::Cursor::new(Vec::new())
}
