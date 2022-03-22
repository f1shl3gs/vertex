use std::{
    cmp,
    path::{Path, PathBuf},
    time::Duration,
};

use crc32fast::Hasher;
use snafu::Snafu;

use super::io::{Filesystem, ProductionFilesystem};

// We don't want data files to be bigger than 128MB, but we might end up overshooting slightly.
pub const DEFAULT_MAX_DATA_FILE_SIZE: u64 = 128 * 1024 * 1024;
// There's no particular reason that _has_ to be 8MB, it's just a simple default we've chosen here.
pub const DEFAULT_MAX_RECORD_SIZE: usize = 8 * 1024 * 1024;
// Using 256KB as it aligns nicely with the I/O size exposed by major cloud providers.  This may not
// be the underlying block size used by the OS, but it still aligns well with what will happen on
// the "backend" for cloud providers, which is simply a useful default for when we want to look at
// buffer throughput and estimate how many IOPS will be consumed, etc.
pub const DEFAULT_WRITE_BUFFER_SIZE: usize = 256 * 1024;

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

#[derive(Debug, Snafu)]
pub enum BuildError {
    #[snafu(display("parameter '{}' was invalid: {}", param_name, reason))]
    InvalidParameter {
        param_name: &'static str,
        reason: &'static str,
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
        let flush_interval = self
            .flush_interval
            .unwrap_or_else(|| Duration::from_millis(500));
        let filesystem = self.filesystem;

        // Validate the input parameters.
        if max_buffer_size == 0 {
            return Err(BuildError::InvalidParameter {
                param_name: "max_buffer_size",
                reason: "cannot be zero",
            });
        }

        if max_data_file_size == 0 {
            return Err(BuildError::InvalidParameter {
                param_name: "max_data_file_size",
                reason: "cannot be zero",
            });
        }

        if max_record_size == 0 {
            return Err(BuildError::InvalidParameter {
                param_name: "max_record_size",
                reason: "cannot be zero",
            });
        }

        if write_buffer_size == 0 {
            return Err(BuildError::InvalidParameter {
                param_name: "write_buffer_size",
                reason: "cannot be zero",
            });
        }

        // The actual on-disk maximum buffer size will be the user-supplied `max_buffer_size`
        // rounded up to the next multiple of `DATA_FILE_TARGET_MAX_SIZE`.  Internally, we'll limit
        // ourselves to `max_buffer_size` rounded _down_ to the next multiple of
        // `DATA_FILE_TARGET_MAX_SIZE`, so that when we have a full data file hanging around
        // mid-read, our actual on-disk usage will match what we've told the user to expect.
        //
        // We also ensure that `max_buffer_size` is at least as big as `DATA_FILE_TARGET_MAX_SIZE`
        // which means the overall minimum on-disk buffer size is 256MB (2x 128MB).
        let max_buffer_size = max_buffer_size - (max_buffer_size % max_data_file_size);
        let max_buffer_size = cmp::max(max_buffer_size, max_data_file_size);

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
