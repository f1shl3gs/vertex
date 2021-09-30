use crc::Crc;
use std::io::{Write, SeekFrom, Seek, Read};
use std::path::{Path, PathBuf};
use std::{io, fs};
use std::fs::{File, metadata};
use std::collections::HashSet;
use crate::events::InternalEvents;
use serde::{Deserialize, Serialize};
use crate::metadata_ext::PortableFileExt;


const FINGERPRINT_CRC: Crc<u64> = Crc::<u64>::new(&crc::CRC_64_ECMA_182);
const LEGACY_FINGERPRINT_CRC: Crc<u64> = Crc::<u64>::new(&crc::CRC_64_XZ);

#[derive(Debug, Clone)]
pub struct Fingerprinter {
    pub strategy: FingerprintStrategy,
    pub max_line_length: usize,
    pub ignore_not_found: bool,
}

#[derive(Debug, Clone)]
pub enum FingerprintStrategy {
    Checksum {
        bytes: usize,
        ignored_header_bytes: usize,
        lines: usize,
    },

    FirstLinesChecksum {
        ignored_header_bytes: usize,
        lines: usize,
    },
    DevInode,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Serialize, Deserialize, Ord, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub enum Fingerprint {
    #[serde(rename = "checksum")]
    BytesChecksum(u64),
    #[serde(alias = "first_line_checksum")]
    FirstLinesChecksum(u64),
    DevInode(u64, u64),
    Unknown(u64),
}

impl Fingerprint {
    pub fn as_legacy(&self) -> u64 {
        match self {
            Fingerprint::BytesChecksum(c) => *c,
            Fingerprint::FirstLinesChecksum(c) => *c,
            Fingerprint::DevInode(dev, ino) => {
                let mut buf = Vec::with_capacity(std::mem::size_of_val(dev) * 2);
                buf.write_all(&dev.to_be_bytes()).expect("writing to array");
                buf.write_all(&ino.to_be_bytes()).expect("writing to array");
                FINGERPRINT_CRC.checksum(&buf[..])
            }
            Fingerprint::Unknown(c) => *c
        }
    }
}

impl From<u64> for Fingerprint {
    fn from(u: u64) -> Self {
        Fingerprint::Unknown(u)
    }
}

impl Fingerprinter {
    pub fn get_fingerprint_of_file(
        &self,
        path: &Path,
        buffer: &mut Vec<u8>,
    ) -> Result<Fingerprint, io::Error> {
        use Fingerprint::*;

        match self.strategy {
            FingerprintStrategy::DevInode => {
                let f = File::open(path)?;
                let dev = f.portable_dev()?;
                let ino = f.portable_ino()?;

                Ok(DevInode(dev, ino))
            }

            FingerprintStrategy::Checksum {
                ignored_header_bytes,
                bytes: _,
                lines,
            } | FingerprintStrategy::FirstLinesChecksum {
                ignored_header_bytes,
                lines,
            } => {
                buffer.resize(self.max_line_length, 0u8);
                let mut fp = fs::File::open(path)?;
                fp.seek(SeekFrom::Start(ignored_header_bytes as u64))?;
                fingerprinter_read_until(fp, b'\n', lines, buffer)?;
                let fingerprint = FINGERPRINT_CRC.checksum(&buffer[..]);
                Ok(FirstLinesChecksum(fingerprint))
            }
        }
    }

    pub fn get_fingerprint_or_log_error(
        &self,
        path: &Path,
        buffer: &mut Vec<u8>,
        known_small_files: &mut HashSet<PathBuf>,
        emitter: &impl InternalEvents,
    ) -> Option<Fingerprint> {
        metadata(path)
            .and_then(|meta| {
                if meta.is_dir() {
                    Ok(None)
                } else {
                    self.get_fingerprint_of_file(path, buffer).map(Some)
                }
            })
            .map_err(|err| match err.kind() {
                io::ErrorKind::UnexpectedEof => {
                    if !known_small_files.contains(path) {
                        emitter.emit_file_checksum_failed(path);
                        known_small_files.insert(path.to_path_buf());
                    }
                }
                io::ErrorKind::NotFound => {
                    if !self.ignore_not_found {
                        emitter.emit_file_fingerprint_read_failed(path, err)
                    }
                }
                _ => {
                    emitter.emit_file_fingerprint_read_failed(path, err)
                }
            })
            .ok()
            .flatten()
    }

    pub fn get_bytes_checksum(
        &self,
        path: &Path,
        buffer: &mut Vec<u8>,
    ) -> Result<Option<Fingerprint>, io::Error> {
        match self.strategy {
            FingerprintStrategy::Checksum {
                bytes,
                ignored_header_bytes,
                lines: _,
            } => {
                buffer.resize(bytes, 0u8);
                let mut f = fs::File::open(path)?;
                f.seek(io::SeekFrom::Start(ignored_header_bytes as u64))?;
                f.read_exact(&mut buffer[..bytes])?;
                let fp = FINGERPRINT_CRC.checksum(&buffer[..]);
                Ok(Some(Fingerprint::BytesChecksum(fp)))
            }

            _ => Ok(None)
        }
    }


    /// Remove this when i figure out how this works
    pub fn get_legacy_checksum(
        &self,
        path: &Path,
        buffer: &mut Vec<u8>,
    ) -> Result<Option<Fingerprint>, io::Error> {
        match self.strategy {
            FingerprintStrategy::Checksum {
                ignored_header_bytes,
                bytes: _,
                lines,
            } | FingerprintStrategy::FirstLinesChecksum {
                ignored_header_bytes,
                lines,
            } => {
                buffer.resize(self.max_line_length, 0u8);
                let mut f = fs::File::open(path)?;
                f.seek(SeekFrom::Start(ignored_header_bytes as u64))?;
                fingerprinter_read_until(f, b'\n', lines, buffer)?;
                let fp = LEGACY_FINGERPRINT_CRC.checksum(&buffer[..]);
                Ok(Some(Fingerprint::FirstLinesChecksum(fp)))
            }

            _ => Ok(None)
        }
    }
}

fn fingerprinter_read_until(
    mut r: impl Read,
    delim: u8,
    mut count: usize,
    mut buf: &mut [u8],
) -> io::Result<()> {
    'main: while !buf.is_empty() {
        let read = match r.read(buf) {
            Ok(0) => return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "EOF reached")),
            Ok(n) => n,
            Err(ref err) if err.kind() == io::ErrorKind::Interrupted => continue,
            Err(err) => return Err(err),
        };

        for (pos, &c) in buf[..read].iter().enumerate() {
            if c != delim {
                continue;
            }

            if count <= 1 {
                for el in &mut buf[(pos + 1)..] {
                    *el = 0;
                }

                break 'main;
            } else {
                count -= 1;
            }
        }

        buf = &mut buf[read..];
    }

    Ok(())
}


#[cfg(test)]
mod test {
    use super::{InternalEvents, FingerprintStrategy, Fingerprinter};
    use std::{collections::HashSet, fs, io::Error, path::Path, time::Duration};
    use tempfile::tempdir;

    #[test]
    fn test_checksum_fingerprint() {
        let fingerprinter = Fingerprinter {
            strategy: FingerprintStrategy::Checksum {
                bytes: 256,
                ignored_header_bytes: 0,
                lines: 1,
            },
            max_line_length: 1024,
            ignore_not_found: false,
        };

        let target_dir = tempdir().unwrap();
        let mut full_line_data = vec![b'x'; 256];
        full_line_data.push(b'\n');
        let not_full_line_data = vec![b'x'; 199];
        let empty_path = target_dir.path().join("empty.log");
        let full_line_path = target_dir.path().join("full_line.log");
        let duplicate_path = target_dir.path().join("duplicate.log");
        let not_full_line_path = target_dir.path().join("not_full_line.log");
        fs::write(&empty_path, &[]).unwrap();
        fs::write(&full_line_path, &full_line_data).unwrap();
        fs::write(&duplicate_path, &full_line_data).unwrap();
        fs::write(&not_full_line_path, &not_full_line_data).unwrap();

        let mut buf = Vec::new();
        assert!(fingerprinter
            .get_fingerprint_of_file(&empty_path, &mut buf)
            .is_err());
        assert!(fingerprinter
            .get_fingerprint_of_file(&full_line_path, &mut buf)
            .is_ok());
        assert!(fingerprinter
            .get_fingerprint_of_file(&not_full_line_path, &mut buf)
            .is_err());
        assert_eq!(
            fingerprinter
                .get_fingerprint_of_file(&full_line_path, &mut buf)
                .unwrap(),
            fingerprinter
                .get_fingerprint_of_file(&duplicate_path, &mut buf)
                .unwrap(),
        );
    }

    #[test]
    fn test_first_line_checksum_fingerprint() {
        let max_line_length = 64;
        let fingerprinter = Fingerprinter {
            strategy: FingerprintStrategy::FirstLinesChecksum {
                ignored_header_bytes: 0,
                lines: 1,
            },
            max_line_length,
            ignore_not_found: false,
        };

        let target_dir = tempdir().unwrap();
        let prepare_test = |file: &str, contents: &[u8]| {
            let path = target_dir.path().join(file);
            fs::write(&path, contents).unwrap();
            path
        };
        let prepare_test_long = |file: &str, amount| {
            prepare_test(
                file,
                b"hello world "
                    .iter()
                    .cloned()
                    .cycle()
                    .clone()
                    .take(amount)
                    .collect::<Box<_>>()
                    .as_ref(),
            )
        };

        let empty = prepare_test("empty.log", b"");
        let incomlete_line = prepare_test("incomlete_line.log", b"missing newline char");
        let one_line = prepare_test("one_line.log", b"hello world\n");
        let one_line_duplicate = prepare_test("one_line_duplicate.log", b"hello world\n");
        let one_line_continued =
            prepare_test("one_line_continued.log", b"hello world\nthe next line\n");
        let different_two_lines = prepare_test("different_two_lines.log", b"line one\nline two\n");

        let exactly_max_line_length =
            prepare_test_long("exactly_max_line_length.log", max_line_length);
        let exceeding_max_line_length =
            prepare_test_long("exceeding_max_line_length.log", max_line_length + 1);
        let incomplete_under_max_line_length_by_one = prepare_test_long(
            "incomplete_under_max_line_length_by_one.log",
            max_line_length - 1,
        );

        let mut buf = Vec::new();
        let mut run = move |path| fingerprinter.get_fingerprint_of_file(path, &mut buf);

        assert!(run(&empty).is_err());
        assert!(run(&incomlete_line).is_err());
        assert!(run(&incomplete_under_max_line_length_by_one).is_err());

        assert!(run(&one_line).is_ok());
        assert!(run(&one_line_duplicate).is_ok());
        assert!(run(&one_line_continued).is_ok());
        assert!(run(&different_two_lines).is_ok());
        assert!(run(&exactly_max_line_length).is_ok());
        assert!(run(&exceeding_max_line_length).is_ok());

        assert_eq!(run(&one_line).unwrap(), run(&one_line_duplicate).unwrap());
        assert_eq!(run(&one_line).unwrap(), run(&one_line_continued).unwrap());

        assert_ne!(run(&one_line).unwrap(), run(&different_two_lines).unwrap());

        assert_eq!(
            run(&exactly_max_line_length).unwrap(),
            run(&exceeding_max_line_length).unwrap()
        );
    }

    #[test]
    fn test_first_two_lines_checksum_fingerprint() {
        let max_line_length = 64;
        let fingerprinter = Fingerprinter {
            strategy: FingerprintStrategy::FirstLinesChecksum {
                ignored_header_bytes: 0,
                lines: 2,
            },
            max_line_length,
            ignore_not_found: false,
        };

        let target_dir = tempdir().unwrap();
        let prepare_test = |file: &str, contents: &[u8]| {
            let path = target_dir.path().join(file);
            fs::write(&path, contents).unwrap();
            path
        };

        let incomlete_lines = prepare_test(
            "incomlete_lines.log",
            b"missing newline char\non second line",
        );
        let two_lines = prepare_test("two_lines.log", b"hello world\nfrom vector\n");
        let two_lines_duplicate =
            prepare_test("two_lines_duplicate.log", b"hello world\nfrom vector\n");
        let two_lines_continued = prepare_test(
            "two_lines_continued.log",
            b"hello world\nfrom vector\nthe next line\n",
        );
        let different_three_lines = prepare_test(
            "different_three_lines.log",
            b"line one\nline two\nine three\n",
        );

        let mut buf = Vec::new();
        let mut run = move |path| fingerprinter.get_fingerprint_of_file(path, &mut buf);

        assert!(run(&incomlete_lines).is_err());

        assert!(run(&two_lines).is_ok());
        assert!(run(&two_lines_duplicate).is_ok());
        assert!(run(&two_lines_continued).is_ok());
        assert!(run(&different_three_lines).is_ok());

        assert_eq!(run(&two_lines).unwrap(), run(&two_lines_duplicate).unwrap());
        assert_eq!(run(&two_lines).unwrap(), run(&two_lines_continued).unwrap());

        assert_ne!(
            run(&two_lines).unwrap(),
            run(&different_three_lines).unwrap()
        );
    }

    #[test]
    fn test_inode_fingerprint() {
        let fingerprinter = Fingerprinter {
            strategy: FingerprintStrategy::DevInode,
            max_line_length: 42,
            ignore_not_found: false,
        };

        let target_dir = tempdir().unwrap();
        let small_data = vec![b'x'; 1];
        let medium_data = vec![b'x'; 256];
        let empty_path = target_dir.path().join("empty.log");
        let small_path = target_dir.path().join("small.log");
        let medium_path = target_dir.path().join("medium.log");
        let duplicate_path = target_dir.path().join("duplicate.log");
        fs::write(&empty_path, &[]).unwrap();
        fs::write(&small_path, &small_data).unwrap();
        fs::write(&medium_path, &medium_data).unwrap();
        fs::write(&duplicate_path, &medium_data).unwrap();

        let mut buf = Vec::new();
        assert!(fingerprinter
            .get_fingerprint_of_file(&empty_path, &mut buf)
            .is_ok());
        assert!(fingerprinter
            .get_fingerprint_of_file(&small_path, &mut buf)
            .is_ok());
        assert_ne!(
            fingerprinter
                .get_fingerprint_of_file(&medium_path, &mut buf)
                .unwrap(),
            fingerprinter
                .get_fingerprint_of_file(&duplicate_path, &mut buf)
                .unwrap()
        );
    }

    #[test]
    fn no_error_on_dir() {
        let target_dir = tempdir().unwrap();
        let fingerprinter = Fingerprinter {
            strategy: FingerprintStrategy::Checksum {
                bytes: 256,
                ignored_header_bytes: 0,
                lines: 1,
            },
            max_line_length: 1024,
            ignore_not_found: false,
        };

        let mut buf = Vec::new();
        let mut small_files = HashSet::new();
        assert!(fingerprinter
            .get_fingerprint_or_log_error(
                &target_dir.path().to_owned(),
                &mut buf,
                &mut small_files,
                &NoErrors,
            )
            .is_none());
    }

    #[derive(Clone)]
    struct NoErrors;

    impl InternalEvents for NoErrors {
        fn emit_file_added(&self, _: &Path) {}

        fn emit_file_resumed(&self, _: &Path, _: u64) {}

        fn emit_file_watch_failed(&self, _: &Path, _: Error) {
            panic!();
        }

        fn emit_file_unwatched(&self, _: &Path) {}

        fn emit_file_deleted(&self, _: &Path) {}

        fn emit_file_delete_failed(&self, _: &Path, _: Error) {
            panic!();
        }

        fn emit_file_fingerprint_read_failed(&self, _: &Path, _: Error) {
            panic!();
        }

        fn emit_file_checkpointed(&self, _: usize, _: Duration) {}

        fn emit_file_checksum_failed(&self, _: &Path) {
            panic!();
        }

        fn emit_file_checkpoint_write_failed(&self, _: Error) {
            panic!();
        }

        fn emit_files_open(&self, _: usize) {}

        fn emit_path_globbing_failed(&self, _: &Path, _: &Error) {}
    }
}
