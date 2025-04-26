use std::path::PathBuf;

use crate::assert_ledger_empty;
use crate::disk::tests::{create_buffer, with_temp_dir};

#[tokio::test]
async fn reader_skip_decode_error() {
    with_temp_dir(async |root: PathBuf| {
        let (mut writer, mut reader, ledger) = create_buffer(root);
        assert_ledger_empty!(ledger);

        writer.write(10.into()).await.unwrap();
        writer.write((11, true).into()).await.unwrap();
        writer.write(12.into()).await.unwrap();

        let mut msg = reader.read().await.unwrap().unwrap();
        msg.acknowledge().await;
        assert_eq!(msg.size, 10);
        assert!(!msg.decode_err);

        let msg = reader.read().await;
        assert!(msg.is_err());

        let mut msg = reader.read().await.unwrap().unwrap();
        msg.acknowledge().await;
        assert_eq!(msg.size, 12);
        assert!(!msg.decode_err);
    })
    .await
}

#[tokio::test]
async fn reader_throws_error_when_record_length_delimiter_is_error() {}

#[tokio::test]
async fn reader_throws_error_when_finished_file_has_truncated_record_data() {}

#[tokio::test]
async fn reader_throws_error_when_record_has_scrambled_data() {}

#[tokio::test]
async fn reader_throws_error_when_record_has_decoding_error() {}

#[tokio::test]
async fn writer_detects_when_last_record_has_scrambled_data() {}

#[tokio::test]
async fn writer_detects_when_last_record_has_invalid_checksum() {}

#[tokio::test]
async fn writer_detects_when_last_record_wasnt_flushed() {}

#[tokio::test]
async fn writer_detects_when_last_record_was_flushed_but_id_wasnt_incremented() {}

#[tokio::test]
async fn reader_throws_error_when_record_is_undecodable() {}
