use std::{
    fs,
    io::{self, BufRead, Seek},
    path::PathBuf,
    time::{
        Duration,
        Instant,
    },
};

use bytes::{Bytes, BytesMut};
use chrono::{DateTime, Utc};
use tracing::{debug};
use flate2::bufread::MultiGzDecoder;

use crate::{Position, ReadFrom};
use crate::buffer::read_until_with_max_size;
use crate::metadata_ext::PortableFileExt;


/// The `Watcher` struct defines the polling based state machine which reads
/// from a file path, transparently updating the underlying file descriptor
/// when the file has been rolled over, as is common for logs.
///
/// The `Watcher` is expected to live for the lifetime of the file path.
/// `Server` is responsible for clearing away `Watchers` which no
/// longer exist
pub struct Watcher {
    pub path: PathBuf,

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
    /// A `Watcher` tracks _only one_ file. This function returns None if
    /// the path does not exist or is not readable by the current process.
    pub fn new(
        path: PathBuf,
        read_from: ReadFrom,
        ignore_before: Option<DateTime<Utc>>,
        max_line_bytes: usize,
        line_delimiter: Bytes,
    ) -> Result<Watcher, io::Error> {
        let f = fs::File::open(&path)?;
        let (devno, ino) = (f.portable_dev()?, f.portable_ino()?);
        let metadata = f.metadata()?;
        let mut reader = io::BufReader::new(f);

        let too_old = if let (Some(ignore_before), Ok(modified_time)) = (
            ignore_before,
            metadata.modified().map(DateTime::<Utc>::from),
        )
        {
            modified_time < ignore_before
        } else {
            false
        };

        let gzipped = is_gzipped(&mut reader)?;

        // Determine the actual position at which we should start reading
        let (reader, position): (Box<dyn BufRead>, Position) = match (gzipped, too_old, read_from) {
            (true, true, _) => {
                debug!("not reading gzipped file older than `ignore_older` {:?}", path);
                // TODO: why null_reader is needed?
                (Box::new(null_reader()), 0)
            }

            (true, _, ReadFrom::Checkpoint(pos)) => {
                debug!("Not re-reading gzipped file with existing stored offset, {:?} at {}", path, pos);
                (Box::new(null_reader()), pos)
            }

            // TODO: This may become the default, leading us to stop reading gzipped files that
            // we were reading before. Should we merge this and the next branch to read
            // compressed file from the beginning even when `read_from = end` ( implicitly
            // via default or explicitly via config)?
            (true, _, ReadFrom::End) => {
                debug!("Can't read from the end of already-compressed file, {:?}", path);
                (Box::new(null_reader()), 0)
            }

            (true, false, ReadFrom::Beginning) => {
                (Box::new(io::BufReader::new(MultiGzDecoder::new(reader))), 0)
            }

            (false, true, _) => {
                let pos = reader.seek(io::SeekFrom::End(0)).unwrap();
                (Box::new(reader), pos)
            }

            (false, false, ReadFrom::Checkpoint(pos)) => {
                let pos = reader.seek(io::SeekFrom::Start(pos)).unwrap();
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

        let ts = metadata.modified()
            .ok()
            .and_then(|mtime| mtime.elapsed().ok())
            .and_then(|diff| Instant::now().checked_sub(diff))
            .unwrap_or_else(Instant::now);

        Ok(Watcher {
            path,
            findable: true,
            reader,
            position,
            devno,
            inode: ino,
            dead: false,
            last_read_attempt: ts,
            last_read_success: ts,
            max_line_bytes,
            line_delimiter,
            buf: BytesMut::new(),
        })
    }

    pub fn update_path(&mut self, path: PathBuf) -> io::Result<()> {
        let f = fs::File::open(&path)?;
        if (f.portable_dev()?, f.portable_ino()?) != (self.devno, self.inode) {
            let mut reader = io::BufReader::new(fs::File::open(&path)?);
            let gzipped = is_gzipped(&mut reader)?;
            let new_reader: Box<dyn BufRead> = if gzipped {
                if self.position != 0 {
                    Box::new(null_reader())
                } else {
                    Box::new(io::BufReader::new(MultiGzDecoder::new(reader)))
                }
            } else {
                reader.seek(io::SeekFrom::Start(self.position))?;
                Box::new(reader)
            };

            self.reader = new_reader;
            self.devno = f.portable_dev()?;
            self.inode = f.portable_ino()?;
        }

        self.path = path;
        Ok(())
    }

    pub fn set_file_findable(&mut self, f: bool) {
        self.findable = f
    }

    pub fn file_findable(&mut self) -> bool {
        self.findable
    }

    pub fn set_dead(&mut self) {
        self.dead = true
    }

    pub fn dead(&self) -> bool {
        self.dead
    }

    pub fn get_file_position(&self) -> Position {
        self.position
    }

    /// Read a single line from the underlying file
    ///
    /// This function will attempt to read a new line from its file, blocking
    /// up to some maximunm but unspecified amount of time. `read_line` will
    /// open a new file handler as needed, transparently to the caller
    pub fn read_line(&mut self) -> io::Result<Option<Bytes>> {
        self.track_read_attempt();

        let reader = &mut self.reader;
        let pos = &mut self.position;
        match read_until_with_max_size(
            reader,
            pos,
            self.line_delimiter.as_ref(),
            &mut self.buf,
            self.max_line_bytes,
        ) {
            Ok(Some(_)) => {
                self.track_read_success();
                Ok(Some(self.buf.split().freeze()))
            }

            Ok(None) => {
                if !self.file_findable() {
                    self.set_dead();
                    // File has been deleted, so return what we have in the buffer, even though
                    // it didn't end with a newline. This is not a perfect signal for when we should
                    // give up waiting for a newline, but it's decent
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
                    self.set_dead();
                }

                Err(err)
            }
        }
    }

    #[inline]
    fn track_read_attempt(&mut self) {
        self.last_read_attempt = Instant::now()
    }

    #[inline]
    fn track_read_success(&mut self) {
        self.last_read_success = Instant::now()
    }

    #[inline]
    pub fn last_read_success(&self) -> Instant {
        self.last_read_success
    }

    #[inline]
    pub fn should_read(&self) -> bool {
        self.last_read_success.elapsed() < Duration::from_secs(10) ||
            self.last_read_attempt.elapsed() > Duration::from_secs(10)
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