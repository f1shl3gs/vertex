use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

use crc32fast::Hasher;

/// Result of checking if a buffer contained a valid record
pub enum RecordStatus {
    /// The record was able to be read from the buffer, and the checksum is valid.
    ///
    /// Contains the ID for the given record
    Valid { id: u64 },

    /// The record was able to be read from the buffer, but the checksum was not valid.
    Corrupted { calculated: u32, actual: u32 },
}

/// Validate Record crc32 + id + payload
pub fn validate_record(buf: &[u8]) -> RecordStatus {
    assert!(buf.len() > 4 + 8);

    let checksum = unsafe { std::ptr::read::<u32>(buf.as_ptr() as *const _) };
    let calculated = {
        let mut hasher = Hasher::new();

        hasher.update(&buf[4..]);
        hasher.finalize()
    };
    if checksum != calculated {
        return RecordStatus::Corrupted {
            calculated,
            actual: checksum,
        };
    }

    let id = unsafe { std::ptr::read(buf.as_ptr().add(4) as *const _) };
    RecordStatus::Valid { id }
}

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),

    Corrupted,

    PartialWrite,
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

/// Validates that the last write in the current writer data file matches the ledger.
pub fn validate_last_write(path: &PathBuf, id: u64) -> Result<(), Error> {
    let size = path.metadata()?.len();
    if size == 0 {
        // If our current chunk file is empty, there's no sense doing this check
        return Ok(());
    }

    let mut file = OpenOptions::new().read(true).open(path)?;
    let position = seek_to_next_record_id(&mut file, id)?;
    if position == size {
        // reach the file end, and we didn't find the record still
        return Ok(());
    }

    let mut len_buf = [0u8; 4 + 4];
    file.read_exact(&mut len_buf)?;
    let length = u32::from_be_bytes((&len_buf[..4]).try_into().unwrap()) as usize;
    let checksum = u32::from_ne_bytes((&len_buf[4..]).try_into().unwrap());

    let mut consumed = 4; // the record length include crc32, so it's 4 not 0
    let mut hasher = Hasher::new();
    let mut buf = vec![0u8; 4096];

    while consumed < length {
        let amount = file.read(&mut buf)?;
        if amount == 0 {
            return Err(Error::PartialWrite);
        }

        hasher.update(&buf[..amount]);
        consumed += amount;
    }

    if checksum != hasher.finalize() {
        return Err(Error::Corrupted);
    }

    Ok(())
}

/// Seeks to where this reader previously left off, and return the remaining size,
/// aka, file_size - position
pub fn seek_to_next_record_id(file: &mut std::fs::File, id: u64) -> Result<u64, Error> {
    let limit = file.metadata()?.len();
    let mut position = 0;

    // length delimiter + crc32 + id
    let mut buf = [0u8; 4 + 4 + 8];

    loop {
        if let Err(err) = file.read_exact(&mut buf) {
            // seek to end, but we still not find the right record
            if err.kind() == std::io::ErrorKind::UnexpectedEof {
                return Ok(limit - position);
            }

            return Err(Error::Io(err));
        }

        let len = u32::from_be_bytes((&buf[..4]).try_into().unwrap()) as u64;
        if len + position > limit {
            return Err(Error::PartialWrite);
        }

        let current = u64::from_ne_bytes((&buf[8..]).try_into().unwrap());
        if current >= id {
            file.seek_relative(-16)?;
            return Ok(limit - position);
        }

        position += len + 4;
        if position > limit {
            // partial write found, and it is safe to ignore when start
            position = limit
        }

        // seek overflow won't cause a error
        file.seek(SeekFrom::Start(position))?;
    }
}
