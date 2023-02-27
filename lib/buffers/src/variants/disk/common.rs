use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use crate::variants::disk::ledger::LEDGER_LEN;
use crate::variants::disk::record::RECORD_HEADER_LEN;
use crc32fast::Hasher;
use thiserror::Error;

use super::io::{Filesystem, ProductionFilesystem};

// We want to ensure a reasonable time before we `fsync`/flush to disk, and 500ms should provide that for non-critical
// workloads.
//
// Practically, it's far more definitive than `disk_v1` which does not definitely `fsync` at all, at least with how we
// have it configured.
pub const DEFAULT_FLUSH_INTERVAL: Duration = Duration::from_millis(500);

// We don't want data files to be bigger than 128MB, but we might end up overshooting slightly.
pub const DEFAULT_MAX_DATA_FILE_SIZE: u64 = 128 * 1024 * 1024;
// There's no particular reason that _has_ to be 8MB, it's just a simple default we've chosen here.
pub const DEFAULT_MAX_RECORD_SIZE: usize = 8 * 1024 * 1024;
// Using 256KB as it aligns nicely with the I/O size exposed by major cloud providers.  This may not
// be the underlying block size used by the OS, but it still aligns well with what will happen on
// the "backend" for cloud providers, which is simply a useful default for when we want to look at
// buffer throughput and estimate how many IOPS will be consumed, etc.
pub const DEFAULT_WRITE_BUFFER_SIZE: usize = 256 * 1024;

// The maximum record size has to be bigger than the record header itself, since we count the record header towards
// sizing/space usage, etc... but we also use the overaligned version here to make sure we're similarly accounting for
// what `rkyv` will do when we serialize a record.
pub const MINIMUM_MAX_RECORD_SIZE: usize = align16(RECORD_HEADER_LEN + 1);

// The alignment used by the record serializer.
const SERIALIZER_ALIGNMENT: usize = 16;
const MAX_ALIGNABLE_AMOUNT: usize = usize::MAX - SERIALIZER_ALIGNMENT;

/// Aligns the given amount to 16.
///
/// This is required due to the overalignment used in record serialization, such that we can correctly determine minimum
/// on-disk sizes for various elements, and account for those in size limits, etc.
pub(crate) const fn align16(amount: usize) -> usize {
    // The amount must be less than `MAX_ALIGNABLE_AMOUNT` otherwise we'll overflow trying to align it, ending up with a
    // nonsensical value.
    assert!(
        amount <= MAX_ALIGNABLE_AMOUNT,
        "`amount` must be less than `MAX_ALIGNABLE_AMOUNT`"
    );

    ((amount + SERIALIZER_ALIGNMENT - 1) / SERIALIZER_ALIGNMENT) * SERIALIZER_ALIGNMENT
}

// We specifically limit ourselves to 0-31 for file IDs in test, because it lets us more quickly
// create/consume the file IDs so we can test edge cases like file ID rollover and "writer is
// waiting to open file that reader is still on".
#[cfg(not(test))]
pub const MAX_FILE_ID: u16 = u16::MAX;
#[cfg(test)]
pub const MAX_FILE_ID: u16 = 6;

pub(crate) fn create_crc32c_hasher() -> Hasher {
    crc32fast::Hasher::new()
}

/// Gets the maximum possible data file size given the type-level numerical
/// limits and buffer invariants.
fn get_maximum_data_file_size() -> u64 {
    let ledger_len: u64 = LEDGER_LEN
        .try_into()
        .expect("Ledger length cannot be greater than `u64`");

    (u64::MAX - ledger_len) / 2
}

/// Gets the minimum buffer size for the given maximum data file size.
///
/// This ensures that we are allowed to store enough bytes on-disk, as the buffer
/// design requires being able to always write to a minimum number of data files,
/// etc. This allow ensures that we're accounting for non-data file disk usage so
/// that we do not overrun the specified maximum buffer size when considering the
/// sum total of files placed on disk.
fn get_minimum_buffer_size(max_data_file_size: u64) -> Option<u64> {
    // We're doing this fallible conversion back-and-forth because we have to interoperate
    // with `u64` and `usize`, and we need to ensure we're not getting values that can't
    // be represented correctly in both types, as well as ensuring we're not implicitly
    // overflowing and generating nonsensical numbers.
    let ledger_len = LEDGER_LEN
        .try_into()
        .expect("Ledger length cannot be greater than `u64`.");

    // We always need to be able to allocate two data files, so the buffer size has to be
    // at least as big as 2x data files at their maximum allowed size, plus an allowance
    // for the size of the ledger state itself.
    max_data_file_size
        .checked_mul(2)
        .and_then(|doubled| doubled.checked_add(ledger_len))
}

#[derive(Debug, Error)]
pub enum BuildError {
    #[error("parameter '{param_name}' was invalid: {reason}")]
    InvalidParameter {
        param_name: &'static str,
        reason: String,
    },
}

/// Buffer configuration.
#[derive(Clone, Debug)]
pub struct DiskBufferConfig<FS> {
    /// Directory where this buffer will write its files.
    ///
    /// Must be unique from all other buffers, whether within the same process or other Vector
    /// processes on the machine.
    pub(crate) data_dir: PathBuf,

    /// Maximum size, in bytes, that the buffer can consume.
    ///
    /// The actual maximum on-disk buffer size is this amount rounded up to the next multiple of
    /// `max_data_file_size`, but internally, the next multiple of `max_data_file_size` when
    /// rounding this amount _down_ is what gets used as the maximum buffer size.
    ///
    /// This ensures that we never use more then the documented "rounded to the next multiple"
    /// amount, as we must account for one full data file's worth of extra data.
    pub(crate) max_buffer_size: u64,

    /// Maximum size, in bytes, to target for each individual data file.
    ///
    /// This value is not strictly obey because we cannot know ahead of encoding/serializing if the
    /// free space a data file has is enough to hold the write.  In other words, we never attempt to
    /// write to a data file if it is as larger or larger than this value, but may write a record
    /// that causes a data file to exceed this value by as much as `max_record_size`.
    pub(crate) max_data_file_size: u64,

    /// Maximum size, in bytes, of an encoded record.
    ///
    /// Any record which, when encoded, is larger than this amount (with a small caveat, see note)
    /// will not be written to the buffer.
    pub(crate) max_record_size: usize,

    /// Size, in bytes, of the writer's internal buffer.
    ///
    /// This buffer is used to coalesce writes to the underlying data file where possible, which in
    /// turn reduces the number of syscalls needed to issue writes to the underlying data file.
    pub(crate) write_buffer_size: usize,

    /// Flush interval for ledger and data files.
    ///
    /// While data is asynchronously flushed by the OS, and the reader/writer can proceed with a
    /// "hard" flush (aka `fsync`/`fsyncdata`), the flush interval effectively controls the
    /// acceptable window of time for data loss.
    ///
    /// In the event that data had not yet been durably written to disk, and Vector crashed, the
    /// amount of data written since the last flush would be lost.
    pub(crate) flush_interval: Duration,

    /// Filesystem implementation for opening data files.
    ///
    /// We allow parameterizing the filesystem implementation for ease of testing.  The "filesystem"
    /// implementation essentially defines how we open and delete data files, as well as the type of
    /// the data file objects we get when opening a data file.
    pub(crate) filesystem: FS,
}

/// Builder for [`DiskBufferConfig`].
#[derive(Clone, Debug)]
pub struct DiskBufferConfigBuilder<FS = ProductionFilesystem>
where
    FS: Filesystem,
{
    pub(crate) data_dir: PathBuf,
    pub(crate) max_buffer_size: Option<u64>,
    pub(crate) max_data_file_size: Option<u64>,
    pub(crate) max_record_size: Option<usize>,
    pub(crate) write_buffer_size: Option<usize>,
    pub(crate) flush_interval: Option<Duration>,
    pub(crate) filesystem: FS,
}

impl DiskBufferConfigBuilder {
    pub fn from_path<P>(data_dir: P) -> DiskBufferConfigBuilder
    where
        P: AsRef<Path>,
    {
        DiskBufferConfigBuilder {
            data_dir: data_dir.as_ref().to_path_buf(),
            max_buffer_size: None,
            max_data_file_size: None,
            max_record_size: None,
            write_buffer_size: None,
            flush_interval: None,
            filesystem: ProductionFilesystem,
        }
    }
}

impl<FS> DiskBufferConfigBuilder<FS>
where
    FS: Filesystem,
{
    /// Sets the maximum size, in bytes, that the buffer can consume.
    ///
    /// The actual maximum on-disk buffer size is this amount rounded up to the next multiple of
    /// `max_data_file_size`, but internally, the next multiple of `max_data_file_size` when
    /// rounding this amount _down_ is what gets used as the maximum buffer size.
    ///
    /// This ensures that we never use more then the documented "rounded to the next multiple"
    /// amount, as we must account for one full data file's worth of extra data.
    ///
    /// Defaults to `usize::MAX`, or effectively no limit.  Due to the internal design of the
    /// buffer, the effective maximum limit is around `max_data_file_size + max_record_size` * 2^16.
    #[allow(dead_code)]
    pub fn max_buffer_size(mut self, amount: u64) -> Self {
        self.max_buffer_size = Some(amount);
        self
    }

    /// Sets the maximum size, in bytes, to target for each individual data file.
    ///
    /// This value is not strictly obey because we cannot know ahead of encoding/serializing if the
    /// free space a data file has is enough to hold the write.  In other words, we never attempt to
    /// write to a data file if it is as larger or larger than this value, but may write a record
    /// that causes a data file to exceed this value by as much as `max_record_size`.
    ///
    /// Defaults to 128MB.
    #[allow(dead_code)]
    pub fn max_data_file_size(mut self, amount: u64) -> Self {
        self.max_data_file_size = Some(amount);
        self
    }

    /// Sets the maximum size, in bytes, of an encoded record.
    ///
    /// Any record which, when encoded, is larger than this amount (with a small caveat, see note)
    /// will not be written to the buffer.
    ///
    /// Defaults to 8MB.
    #[allow(dead_code)]
    pub fn max_record_size(mut self, amount: usize) -> Self {
        self.max_record_size = Some(amount);
        self
    }

    /// Size, in bytes, of the writer's internal buffer.
    ///
    /// This buffer is used to coalesce writes to the underlying data file where possible, which in
    /// turn reduces the number of syscalls needed to issue writes to the underlying data file.
    ///
    /// Defaults to 256KB.
    #[allow(dead_code)]
    pub fn write_buffer_size(mut self, amount: usize) -> Self {
        self.write_buffer_size = Some(amount);
        self
    }

    /// Sets the flush interval for ledger and data files.
    ///
    /// While data is asynchronously flushed by the OS, and the reader/writer can proceed with a
    /// "hard" flush (aka `fsync`/`fsyncdata`), the flush interval effectively controls the
    /// acceptable window of time for data loss.
    ///
    /// In the event that data had not yet been durably written to disk, and Vector crashed, the
    /// amount of data written since the last flush would be lost.
    ///
    /// Defaults to 500ms.
    #[allow(dead_code)]
    pub fn flush_interval(mut self, interval: Duration) -> Self {
        self.flush_interval = Some(interval);
        self
    }

    /// Filesystem implementation for opening data files.
    ///
    /// We allow parameterizing the filesystem implementation for ease of testing.  The "filesystem"
    /// implementation essentially defines how we open and delete data files, as well as the type of
    /// the data file objects we get when opening a data file.
    ///
    /// Defaults to a Tokio-backed implementation.
    #[allow(dead_code)]
    pub fn filesystem<FS2>(self, filesystem: FS2) -> DiskBufferConfigBuilder<FS2>
    where
        FS2: Filesystem,
    {
        DiskBufferConfigBuilder {
            data_dir: self.data_dir,
            max_buffer_size: self.max_buffer_size,
            max_data_file_size: self.max_data_file_size,
            max_record_size: self.max_record_size,
            write_buffer_size: self.write_buffer_size,
            flush_interval: self.flush_interval,
            filesystem,
        }
    }

    /// Consumes this builder and constructs a `DiskBufferConfig`.
    pub fn build(self) -> Result<DiskBufferConfig<FS>, BuildError> {
        let max_buffer_size = self.max_buffer_size.unwrap_or(u64::MAX);
        let max_data_file_size = self
            .max_data_file_size
            .unwrap_or(DEFAULT_MAX_DATA_FILE_SIZE);
        let max_record_size = self.max_record_size.unwrap_or(DEFAULT_MAX_RECORD_SIZE);
        let write_buffer_size = self.write_buffer_size.unwrap_or(DEFAULT_WRITE_BUFFER_SIZE);
        let flush_interval = self.flush_interval.unwrap_or(DEFAULT_FLUSH_INTERVAL);
        let filesystem = self.filesystem;

        // Validate the input parameters.
        if max_data_file_size == 0 {
            return Err(BuildError::InvalidParameter {
                param_name: "max_data_file_size",
                reason: "cannot be zero".to_string(),
            });
        }

        let data_file_size_mechanical_limit = get_maximum_data_file_size();
        if max_data_file_size > data_file_size_mechanical_limit {
            return Err(BuildError::InvalidParameter {
                param_name: "max_data_file_size",
                reason: format!(
                    "cannot be greater than {} bytes",
                    data_file_size_mechanical_limit
                ),
            });
        }

        let minimum_buffer_size = match get_minimum_buffer_size(max_data_file_size) {
            Some(value) => value,
            None => {
                unreachable!("maximum data file size should be correctly limited at this point")
            }
        };

        if max_buffer_size < minimum_buffer_size {
            return Err(BuildError::InvalidParameter {
                param_name: "max_buffer_size",
                reason: format!(
                    "must be greater than or equal to {} bytes",
                    minimum_buffer_size
                ),
            });
        }

        if max_record_size == 0 {
            return Err(BuildError::InvalidParameter {
                param_name: "max_record_size",
                reason: "cannot be zero".to_string(),
            });
        }

        if max_record_size <= MINIMUM_MAX_RECORD_SIZE {
            return Err(BuildError::InvalidParameter {
                param_name: "max_record_size",
                reason: format!(
                    "must be greater than or equal to {} bytes",
                    MINIMUM_MAX_RECORD_SIZE,
                ),
            });
        }

        let max_record_size_converted = match u64::try_from(max_record_size) {
            Ok(value) => value,
            Err(_) => {
                return Err(BuildError::InvalidParameter {
                    param_name: "max_record_size",
                    reason: "must be less than 2^64 bytes".to_string(),
                })
            }
        };

        if max_record_size_converted > max_data_file_size {
            return Err(BuildError::InvalidParameter {
                param_name: "max_record_size",
                reason: "must be less than or equal to `max_data_file_size`".to_string(),
            });
        }

        if write_buffer_size == 0 {
            return Err(BuildError::InvalidParameter {
                param_name: "write_buffer_size",
                reason: "cannot be zero".to_string(),
            });
        }

        // Users configure the `max_size` of their disk buffers, which translates to the
        // `max_buffer_size` field here, and represents the maximum desired size of a disk
        // buffer in terms of on-disk usage. In order to meet this request, we do a few
        // things internally and also enforce a lower bound on `max_buffer_size` to ensure
        // we can commit to respecting the communicated maximum buffer size.
        //
        // Internally, we track the current buffer size as a function of the sum of the
        // size of all unacknowledged records.  This means, simply, that if 100 records
        // are written that consume 1KB a piece, our current buffer size should be around
        // 100KB, and as those records are read and acknowledged, the current buffer size
        // would drop by 1KB for each of them until eventually it went back down to zero.
        //
        // One of the design invariants around data files is that they are written to until
        // they reach the maximum data file size, such that they are guaranteed to never be
        // greater in size than `max_data_file_size`. This is coupled with the fact that a
        // data file cannot be deleted from disk until all records written to it have been
        // read _and_ acknowledged.
        //
        // Together, this means that we need to set a lower bound of 2*`max_data_file_size
        // for `max_buffer_size`.
        //
        // First, given the "data file keeps getting written to until we reach its max size"
        // invariant, we know that in order to commit to the on-disk buffer size not exceeding
        // `max_buffer_size`, the value must be at least as much as a single full data file,
        // aka `max_data_file_size`.
        //
        // Secondly, we also want to ensure that the writer can make progress as the reader
        // makes progress. If the maximum buffer size was equal to the maximum data file size,
        // the writer would be stalled as soon as the data file reached the maximum size,
        // until the reader was able to fully read and acknowledge all records, and thus
        // delete the data file from disk. If we instead require that the maximum buffer
        // size exceeds `max_data_file_size`, this allows us to open the next data file and
        // start writing to it up until the maximum buffer size.
        //
        // Since we could essentially read and acknowledge all but the last remaining record
        // in a data file, this would imply we gave the writer the ability to write that much
        // more data, which means we would need at least double the maximum data file size in
        // order to support the writer being able to make progress in the aforementioned
        // situation.
        //
        // Finally, we come to this calculation. Since the logic dictates that we essentially
        // require at least one extra data file past the minimum of one, we need to use an
        // _internal_ maximum buffer size of `max_buffer_size` - `max_data_file_size`, so that
        // as the reader makes progress, the writer never is led to believe it can create
        // another data file such that the number of active data files, multiplied by
        // `max_data_file_size`, would exceed `max_buffer_size`.
        let max_buffer_size = max_buffer_size - max_data_file_size;

        Ok(DiskBufferConfig {
            data_dir: self.data_dir,
            max_buffer_size,
            max_data_file_size,
            max_record_size,
            write_buffer_size,
            flush_interval,
            filesystem,
        })
    }
}
