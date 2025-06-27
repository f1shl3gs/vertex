use std::path::PathBuf;

use tokio_test::task::spawn;
use tokio_test::{assert_pending, assert_ready};

use crate::assert_ledger_empty;
use crate::disk::Config;
use crate::disk::tests::{create_buffer, create_buffer_with_config, with_temp_dir};

#[tokio::test]
async fn pending_read_returns_none_when_writer_closed_with_unflushed_write() {
    with_temp_dir(async |root: PathBuf| {
        let (mut tx, mut rx, ledger) = create_buffer(root.clone());
        assert_ledger_empty!(ledger);

        let mut read = spawn(rx.read());
        assert_pending!(read.poll());

        tx.write(100.into()).await.unwrap();

        drop(tx);
        drop(ledger);

        tokio::task::yield_now().await;

        let path = root.join(format!("{:04x}.chunk", 0));
        let len = path.metadata().unwrap().len();
        assert_eq!(len, 116);

        assert!(read.is_woken());
        let result = assert_ready!(read.poll()).unwrap();
        assert_eq!(result, Some(100.into()));
    })
    .await;
}

#[tokio::test]
async fn file_id_wraps_around_when_max_file_id_hit() {
    with_temp_dir(async |root: PathBuf| {
        let (mut tx, mut rx, ledger) = create_buffer_with_config(Config {
            root: root.clone(),
            max_record_size: 100,
            max_chunk_size: 100,
            max_buffer_size: 10000,
        });
        assert_ledger_empty!(ledger);

        for _ in 0..8 {
            tx.write(80.into()).await.unwrap();
        }

        for i in 0..8 {
            let path = root.join(format!("{i:04x}.chunk"));
            assert!(path.exists());
        }

        rx.read().await.unwrap();
        rx.read().await.unwrap();

        for i in 0..8 {
            let path = root.join(format!("{i:04x}.chunk"));
            assert!(path.exists());
        }
    })
    .await;
}

#[tokio::test]
async fn writer_stops_when_hitting_file_that_reader_is_still_open() {
    with_temp_dir(async |root: PathBuf| {
        let (mut tx, mut rx, ledger) = create_buffer_with_config(Config {
            root: root.clone(),
            max_record_size: 100,
            max_chunk_size: 100,
            max_buffer_size: 1000,
        });
        assert_ledger_empty!(ledger);

        let reader_path = tx.config().chunk_path(ledger.get_current_reader_file_id());
        assert!(reader_path.exists());

        for _ in 0..8 {
            tx.write(80.into()).await.unwrap();
        }

        let mut write = spawn(async { tx.write(80.into()).await.unwrap() });
        assert_pending!(write.poll());

        // first read, this won't move reader_id
        let mut msg = rx.read().await.unwrap().unwrap();
        msg.acknowledge().await;

        // second read
        let mut msg = rx.read().await.unwrap().unwrap();
        msg.acknowledge().await;

        write.await;
    })
    .await;
}

#[tokio::test]
async fn reader_still_works_when_record_id_wraps_around() {
    with_temp_dir(async |root: PathBuf| {
        let (_, _, ledger) = create_buffer(root.clone());
        ledger.state().set_writer_next_record_id(u64::MAX);
        ledger.state().set_reader_last_record_id(u64::MAX - 1);

        ledger.flush().unwrap();
        drop(ledger);

        tokio::task::yield_now().await;

        let (mut tx, mut rx, ledger) = create_buffer_with_config(Config {
            root,
            max_record_size: 100,
            max_chunk_size: 100,
            max_buffer_size: 1000,
        });

        // Now we do two writes: one which uses u64::MAX, and another which will get the
        // the rolled over value and go back to 0
        let next_record_id = ledger.get_next_write_record_id();
        assert_eq!(next_record_id, u64::MAX);
        tx.write(10.into()).await.unwrap();

        let next_record_id = ledger.get_next_write_record_id();
        assert_eq!(next_record_id, 0);
        tx.write(10.into()).await.unwrap();

        let mut msg = rx.read().await.unwrap().unwrap();
        msg.acknowledge().await;
        assert_eq!(ledger.last_reader_record_id(), u64::MAX);

        let mut msg = rx.read().await.unwrap().unwrap();
        msg.acknowledge().await;
        assert_eq!(ledger.last_reader_record_id(), 0);
    })
    .await;
}

#[tokio::test]
async fn reader_deletes_chunk_file_around_record_id_wrap_around() {
    with_temp_dir(async |root: PathBuf| {
        let (_, _, ledger) = create_buffer(root.clone());
        assert_ledger_empty!(ledger);

        ledger.state().set_writer_next_record_id(u64::MAX);
        ledger.state().set_reader_last_record_id(u64::MAX - 1);

        ledger.flush().unwrap();
        drop(ledger);
        tokio::task::yield_now().await;

        let (mut tx, mut rx, ledger) = create_buffer_with_config(Config {
            root,
            max_record_size: 100,
            max_chunk_size: 200,
            max_buffer_size: 16 * 200,
        });
        assert_ledger_empty!(ledger);
        assert_eq!(ledger.state().get_next_write_record_id(), u64::MAX);
        assert_eq!(ledger.state().get_last_read_record_id(), u64::MAX - 1);

        // after 3 writes, we moved to next chunk
        tx.write(84.into()).await.unwrap();
        tx.write(84.into()).await.unwrap();
        tx.write(84.into()).await.unwrap();

        assert_eq!(ledger.get_buffer_records(), 3);
        assert_eq!(ledger.get_buffer_bytes(), 300);

        assert!(tx.config().chunk_path(0).exists());
        assert!(tx.config().chunk_path(1).exists());
        assert!(!tx.config().chunk_path(2).exists());

        let mut msg = rx.read().await.unwrap().unwrap();
        msg.acknowledge().await;
        let mut msg = rx.read().await.unwrap().unwrap();
        msg.acknowledge().await;
        assert!(tx.config().chunk_path(0).exists());
        assert!(tx.config().chunk_path(1).exists());
        assert!(!tx.config().chunk_path(2).exists());

        let mut msg = rx.read().await.unwrap().unwrap();
        msg.acknowledge().await;
        assert!(!tx.config().chunk_path(0).exists());
        assert!(tx.config().chunk_path(1).exists());
        assert!(!tx.config().chunk_path(2).exists());
    })
    .await
}

#[tokio::test]
async fn writer_waits_for_reader_after_validate_last_write_fails_and_chunk_file_skip_triggered() {
    // When we initialize a buffer, if the writer previously left off on a partially-filled
    // chunk file, we load that chunk file and do a simple check to make sure the last
    // record in the file is valid. If it's not valid, we consider that chunk file corrupted
    // and skip to the next chunk file. This is intended to limit us writing records to a
    // chunk file that the reader is going skip the reset of when it detects a bad/corrupted
    // record
    with_temp_dir(async |root: PathBuf| {
        let (_tx, _rx, ledger) = create_buffer_with_config(Config {
            root,
            max_record_size: 100,
            max_chunk_size: 200,
            max_buffer_size: 2000,
        });
        assert_ledger_empty!(ledger);
    })
    .await
}

#[tokio::test]
async fn writer_updates_ledger_when_buffered_writer_reports_implicit_flush() {}

#[tokio::test]
async fn reader_writer_positions_aligned_through_multiple_files_and_records() {}
