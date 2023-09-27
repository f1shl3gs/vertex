use std::{
    io::{self, Cursor},
    path::Path,
    sync::Arc,
};

use async_trait::async_trait;
use tokio::io::DuplexStream;

use super::{
    io::{AsyncFile, Metadata, ProductionFilesystem, ReadableMemoryMap, WritableMemoryMap},
    Buffer, DiskBufferConfigBuilder, Ledger, Reader, Writer,
};
use crate::encoding::FixedEncodable;
use crate::variants::disk::{common::align16, record::RECORD_HEADER_LEN};
use crate::{buffer_usage_data::BufferUsageHandle, Bufferable};

type FilesystemUnderTest = ProductionFilesystem;

mod acknowledgements;
mod basic;
mod invariants;
mod known_errors;
mod model;
mod record;
mod size_limits;

#[async_trait]
impl AsyncFile for DuplexStream {
    async fn metadata(&self) -> io::Result<Metadata> {
        Ok(Metadata { len: 0 })
    }

    async fn sync_all(&self) -> io::Result<()> {
        Ok(())
    }
}

#[async_trait]
impl AsyncFile for Cursor<Vec<u8>> {
    async fn metadata(&self) -> io::Result<Metadata> {
        Ok(Metadata { len: 0 })
    }

    async fn sync_all(&self) -> io::Result<()> {
        Ok(())
    }
}

impl ReadableMemoryMap for Vec<u8> {}

impl WritableMemoryMap for Vec<u8> {
    fn flush(&self) -> io::Result<()> {
        Ok(())
    }
}

#[macro_export]
macro_rules! assert_buffer_is_empty {
    ($ledger:expr) => {
        assert_eq!(
            $ledger.get_total_records(),
            0,
            "ledger should have 0 records, but had {}",
            $ledger.get_total_records()
        );
        assert_eq!(
            $ledger.get_total_buffer_size(),
            0,
            "ledger should have 0 bytes, but had {} bytes",
            $ledger.get_total_buffer_size()
        );
    };
}

#[macro_export]
macro_rules! assert_buffer_records {
    ($ledger:expr, $record_count:expr) => {
        assert_eq!(
            $ledger.get_total_records(),
            $record_count as u64,
            "ledger should have {} records, but had {}",
            $record_count,
            $ledger.get_total_records()
        );
    };
}

#[macro_export]
macro_rules! assert_buffer_size {
    ($ledger:expr, $record_count:expr, $buffer_size:expr) => {
        assert_eq!(
            $ledger.get_total_records(),
            $record_count as u64,
            "ledger should have {} records, but had {}",
            $record_count,
            $ledger.get_total_records()
        );
        assert_eq!(
            $ledger.get_total_buffer_size(),
            $buffer_size as u64,
            "ledger should have {} bytes, but had {} bytes",
            $buffer_size,
            $ledger.get_total_buffer_size()
        );
    };
}

#[macro_export]
macro_rules! assert_reader_writer_file_positions {
    ($ledger:expr, $reader:expr, $writer:expr) => {{
        let (reader, writer) = $ledger.get_current_reader_writer_file_id();
        assert_eq!(
            ($reader) as u16,
            reader,
            "expected reader file ID of {}, got {} instead",
            ($reader),
            reader
        );
        assert_eq!(
            ($writer) as u16,
            writer,
            "expected writer file ID of {}, got {} instead",
            ($writer),
            writer
        );
    }};
}

#[macro_export]
macro_rules! assert_enough_bytes_written {
    // 8 for length delimit
    // 4 for Record's checksum
    // 8 for Record's id
    // 4 for Record's metadata
    ($written:expr, $record:expr) => {
        let encoded_size = $record.encoded_len();

        assert_eq!(
            $written,
            8 + 4 + 8 + 4 + encoded_size,
            "{} == 8 + 4 + 8 + 4 + {}",
            $written,
            encoded_size
        );
    };
}

#[macro_export]
macro_rules! set_data_file_length {
    ($path:expr, $start_len:expr, $target_len:expr) => {{
        let mut data_file = OpenOptions::new()
            .write(true)
            .open(&$path)
            .await
            .expect("open should not fail");

        // Just to make sure the data file matches our expected state before futzing with it.
        let metadata = data_file
            .metadata()
            .await
            .expect("metadata should not fail");
        assert_eq!(
            ($start_len) as u64,
            metadata.len(),
            "expected data file to be {} bytes long, but was actually {} bytes long",
            ($start_len) as u64,
            metadata.len()
        );

        data_file
            .set_len(($target_len) as u64)
            .await
            .expect("truncate should not fail");
        data_file.flush().await.expect("flush should not fail");
        data_file.sync_all().await.expect("sync should not fail");
        drop(data_file);
    }};
}

pub(crate) async fn create_default_buffer<P, R>(
    data_dir: P,
) -> (
    Writer<R, FilesystemUnderTest>,
    Reader<R, FilesystemUnderTest>,
    Arc<Ledger<FilesystemUnderTest>>,
)
where
    P: AsRef<Path>,
    R: Bufferable,
{
    let config = DiskBufferConfigBuilder::from_path(data_dir)
        .build()
        .expect("creating buffer should not fail");
    let usage_handle = BufferUsageHandle::noop();
    Buffer::from_config_inner(config, usage_handle)
        .await
        .expect("should not fail to create buffer")
}

pub(crate) async fn create_default_buffer_with_usage<P, R>(
    data_dir: P,
) -> (
    Writer<R, FilesystemUnderTest>,
    Reader<R, FilesystemUnderTest>,
    Arc<Ledger<FilesystemUnderTest>>,
    BufferUsageHandle,
)
where
    P: AsRef<Path>,
    R: Bufferable,
{
    let config = DiskBufferConfigBuilder::from_path(data_dir)
        .build()
        .expect("creating buffer should not fail");
    let usage_handle = BufferUsageHandle::noop();
    let (writer, reader, ledger) = Buffer::from_config_inner(config, usage_handle.clone())
        .await
        .expect("should not fail to create buffer");
    (writer, reader, ledger, usage_handle)
}

pub(crate) async fn create_buffer_with_max_buffer_size<P, R>(
    data_dir: P,
    max_buffer_size: u64,
) -> (
    Writer<R, FilesystemUnderTest>,
    Reader<R, FilesystemUnderTest>,
    Arc<Ledger<FilesystemUnderTest>>,
)
where
    P: AsRef<Path>,
    R: Bufferable,
{
    // We override `max_buffer_size` directly because otherwise `build` has built-in logic that
    // ensures it is a minimum size related to the data file size limit, etc.
    let mut config = DiskBufferConfigBuilder::from_path(data_dir)
        .build()
        .expect("creating buffer should not fail");
    config.max_buffer_size = max_buffer_size;
    let usage_handle = BufferUsageHandle::noop();

    Buffer::from_config_inner(config, usage_handle)
        .await
        .expect("should not fail to create buffer")
}

pub(crate) async fn create_buffer_with_max_record_size<P, R>(
    data_dir: P,
    max_record_size: usize,
) -> (
    Writer<R, FilesystemUnderTest>,
    Reader<R, FilesystemUnderTest>,
    Arc<Ledger<FilesystemUnderTest>>,
)
where
    P: AsRef<Path>,
    R: Bufferable,
{
    let config = DiskBufferConfigBuilder::from_path(data_dir)
        .max_record_size(max_record_size)
        .build()
        .expect("creating buffer should not fail");
    let usage_handle = BufferUsageHandle::noop();

    Buffer::from_config_inner(config, usage_handle)
        .await
        .expect("should not fail to create buffer")
}

pub(crate) async fn create_buffer_with_max_data_file_size<P, R>(
    data_dir: P,
    max_data_file_size: u64,
) -> (
    Writer<R, FilesystemUnderTest>,
    Reader<R, FilesystemUnderTest>,
    Arc<Ledger<FilesystemUnderTest>>,
)
where
    P: AsRef<Path>,
    R: Bufferable,
{
    let max_record_size = usize::try_from(max_data_file_size).unwrap();

    let config = DiskBufferConfigBuilder::from_path(data_dir)
        .max_data_file_size(max_data_file_size)
        .max_record_size(max_record_size)
        .build()
        .expect("creating buffer should not fail");
    let usage_handle = BufferUsageHandle::noop();

    Buffer::from_config_inner(config, usage_handle)
        .await
        .expect("should not fail to create buffer")
}

pub(crate) async fn create_buffer_with_write_buffer_size<P, R>(
    data_dir: P,
    write_buffer_size: usize,
) -> (
    Writer<R, FilesystemUnderTest>,
    Reader<R, FilesystemUnderTest>,
    Arc<Ledger<FilesystemUnderTest>>,
)
where
    P: AsRef<Path>,
    R: Bufferable,
{
    let config = DiskBufferConfigBuilder::from_path(data_dir)
        .write_buffer_size(write_buffer_size)
        .build()
        .expect("creating buffer should not fail");
    let usage_handle = BufferUsageHandle::noop();

    Buffer::from_config_inner(config, usage_handle)
        .await
        .expect("should not fail to create buffer")
}

pub(crate) fn get_corrected_max_record_size<T>(payload: &T) -> usize
where
    T: FixedEncodable,
{
    let payload_len = payload
        .encoded_size()
        .expect("All test record types must return a valid encoded size.");
    let total = RECORD_HEADER_LEN + payload_len;

    align16(total)
}

pub(crate) fn get_minimum_data_file_size_for_record_payload<T>(payload: &T) -> u64
where
    T: FixedEncodable,
{
    // This is just the maximum record size, compensating for the record header length.
    let max_record_size = get_corrected_max_record_size(payload);
    u64::try_from(max_record_size).unwrap()
}
