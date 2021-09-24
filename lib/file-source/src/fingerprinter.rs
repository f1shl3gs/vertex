use crc::Crc;
use std::io::{Write, SeekFrom, Seek};
use std::path::{Path, PathBuf};
use std::{io, fs};
use std::fs::File;
use std::collections::HashSet;


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
pub enum FileFingerprint {
    #[serde(rename = "checksum")]
    BytesChecksum(u64),
    #[serde(alias = "first_line_checksum")]
    FirstLinesChecksum(u64),
    DevInode(u64, u64),
    Unknown(u64),
}

impl FileFingerprint {
    pub fn as_legacy(&self) -> u64 {
        match self {
            FileFingerprint::BytesChecksum(c) => *c,
            FileFingerprint::FirstLinesChecksum(c) => *c,
            FileFingerprint::DevInode(dev, ino) => {
                let mut buf = Vec::with_capacity(std::mem::size_of_val(dev) * 2);
                buf.write_all(&dev.to_be_bytes()).expect("writing to array");
                buf.write_all(&ino.to_be_bytes()).expect("writing to array");
                FINGERPRINT_CRC.checksum(&buf[..])
            }
            FileFingerprint::Unknown(c) => *c
        }
    }
}

impl From<u64> for FileFingerprint {
    fn from(u: u64) -> Self {
        FileFingerprint::Unknown(u)
    }
}

impl Fingerprinter {
    pub fn get_fingerprint_of_file(
        &self,
        path: &Path,
        buffer: &mut Vec<u8>,
    ) -> Result<FileFingerprint, io::Error> {
        use FileFingerprint::*;

        match self.strategy {
            FingerprintStrategy::DevInode => {
                let file_handle = File::open(path)?;
                let dev = file_handle.portable_dev()?;
                let ino = file_handle.portable_ino()?;

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
    ) -> Option<FileFingerprint> {

    }
}
