use std::io::Write;
use std::mem;

use crc32fast::Hasher;

use super::{common::align16, ser::DeserializeError};

pub const RECORD_HEADER_LEN: usize = align16(mem::size_of::<ArchivedRecord<'_>>() + 8);

/// Result of checking if a buffer contained a valid record.
pub enum RecordStatus {
    /// The record was able to be read from the buffer, and the checksum is valid.
    ///
    /// Contains the ID for the given record, as well as the metadata.
    Valid { id: u64, metadata: u32 },

    /// The record was able to be read from the buffer, but the checksum was not valid.
    Corrupted { calculated: u32, actual: u32 },

    /// The record was not able to be read from the buffer due to an error during deserialization.
    FailedDeserialization(DeserializeError),
}

/// Record container.
///
/// [`Record`] encapsulates the encoded form of a record written into the buffer.  It is a simple wrapper that
/// carries only the necessary metadata: the record checksum, and a record ID used internally for
/// properly tracking the state of the reader and writer.
///
/// # Warning
///
/// - Do not add fields to this struct.
/// - Do not remove fields from this struct.
/// - Do not change the type of fields in this struct.
/// - Do not change the order of fields this struct.
///
/// Doing so will change the serialized representation.  This will break things.
///
/// Do not do any of the listed things unless you _absolutely_ know what you're doing. :)
pub struct Record<'a> {
    /// The checksum of the record.
    ///
    /// The checksum is CRC32C(BE(id) + BE(metadata) + payload), where BE(x) returns a byte slice of
    /// the given integer in big endian format.
    pub(super) checksum: u32,

    /// The record ID.
    ///
    /// This is monotonic across records.
    id: u64,

    /// The record metadata.
    ///
    /// Based on `Encodable::Metadata`.
    pub(super) metadata: u32,

    /// The record payload.
    ///
    /// This is the encoded form of the actual record itself.
    payload: &'a [u8],
}

impl<'a> Record<'a> {
    /// Creates a [`Record`] from the ID and payload, and calculates the checksum.
    pub fn with_checksum(id: u64, metadata: u32, payload: &'a [u8], checksummer: &Hasher) -> Self {
        let checksum = generate_checksum(checksummer, id, metadata, payload);

        Self {
            checksum,
            id,
            metadata,
            payload,
        }
    }

    /// serialize the Record and return the bytes write to the writer.
    pub fn serialize<W: Write>(&self, w: &mut W) -> std::io::Result<usize> {
        let len = 4 + 8 + 4 + self.payload.len();

        w.write_all(&len.to_be_bytes())?;
        w.write_all(&self.checksum.to_ne_bytes())?;
        w.write_all(&self.id.to_ne_bytes())?;
        w.write_all(&self.metadata.to_ne_bytes())?;
        w.write_all(self.payload)?;

        Ok(len + 8)
    }
}

/// `ArchivedRecord` is a wrapper for serialized data of [`Record`]
pub struct ArchivedRecord<'a>(&'a [u8]);

impl<'a> ArchivedRecord<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self(data)
    }

    pub fn try_new(data: &'a [u8]) -> Result<Self, DeserializeError> {
        if data.len() < 4 + 8 + 4 {
            return Err(DeserializeError::TooShort);
        }

        Ok(ArchivedRecord(data))
    }

    /// Gets the checksum of this record.
    #[inline]
    pub fn checksum(&self) -> u32 {
        unsafe { std::ptr::read(self.0.as_ptr() as *const _) }
    }

    /// Gets the metadata of this record.
    #[inline]
    pub fn metadata(&self) -> u32 {
        unsafe { std::ptr::read(self.0.as_ptr().add(4 + 8) as *const _) }
    }

    /// Gets the payload of this record.
    #[inline]
    pub fn id(&self) -> u64 {
        unsafe { std::ptr::read(self.0.as_ptr().add(4) as *const _) }
    }

    pub fn payload(&self) -> &[u8] {
        &self.0[4 + 8 + 4..]
    }

    /// Verifies if the stored checksum of this record matches the record itself.
    pub fn verify_checksum(&self, checksummer: &Hasher) -> RecordStatus {
        let checksum = self.checksum();
        let id = self.id();
        let metadata = self.metadata();

        let calculated = {
            let mut checksummer = checksummer.clone();
            checksummer.reset();

            checksummer.update(&self.0[4..]);
            checksummer.finalize()
        };

        if checksum == calculated {
            RecordStatus::Valid { id, metadata }
        } else {
            RecordStatus::Corrupted {
                calculated,
                actual: checksum,
            }
        }
    }
}

fn generate_checksum(checksummer: &Hasher, id: u64, metadata: u32, payload: &[u8]) -> u32 {
    let mut checksummer = checksummer.clone();
    checksummer.reset();

    checksummer.update(&id.to_ne_bytes()[..]);
    checksummer.update(&metadata.to_ne_bytes()[..]);
    checksummer.update(payload);
    checksummer.finalize()
}

/// Checks whether the given buffer contains a valid [`Record`] archive.
///
/// The record archive is assumed to have been serialized as the very last item in `buf`, and
/// it is also assumed that the provided `buf` has an alignment of 8 bytes.
///
/// If a record archive was able to be read from the buffer, then the status will indicate whether
/// or not the checksum in the record matched the recalculated checksum.  Otherwise, the
/// deserialization error encounted will be provided, which describes the error in a more verbose,
/// debugging-oriented fashion.
#[cfg_attr(test, instrument(skip_all, level = "trace"))]
pub fn validate_record_archive(buf: &[u8], checksummer: &Hasher) -> RecordStatus {
    match ArchivedRecord::try_new(buf) {
        Ok(archive) => archive.verify_checksum(checksummer),
        Err(err) => RecordStatus::FailedDeserialization(err),
    }
}

pub fn validate_record_archive_with_length(buf: &[u8], checksummer: &Hasher) -> RecordStatus {
    let len = usize::from_be_bytes(buf[..8].try_into().expect("extract size part success"));
    if buf.len() < len + 8 {
        return RecordStatus::FailedDeserialization(DeserializeError::TooShort);
    }

    validate_record_archive(&buf[8..8 + len], checksummer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archived_record() {
        let payload = "ffff".as_bytes();
        let hasher = Hasher::new();
        let record = Record::with_checksum(1, 2, payload, &hasher);
        let checksum = record.checksum;

        let mut buf = Vec::new();
        record.serialize(&mut buf).expect("serialize success");

        let archived = ArchivedRecord::try_new(&buf.as_slice()[8..]).unwrap();
        let record_status = archived.verify_checksum(&hasher);
        assert!(matches!(record_status, RecordStatus::Valid { .. }));

        assert_eq!(archived.checksum(), checksum, "checksum is changed");
        assert_eq!(archived.id(), record.id);
        assert_eq!(archived.metadata(), record.metadata);
        assert_eq!(archived.payload(), record.payload);
    }
}
