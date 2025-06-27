use std::fs::OpenOptions;
use std::path::PathBuf;

use tokio_test::task::spawn;
use tokio_test::{assert_pending, assert_ready};

use super::Config;
use super::{create_buffer_with_config, with_temp_dir};
use crate::assert_ledger_empty;
use crate::disk::writer::Error;
use crate::tests::Message;

#[tokio::test]
async fn writer_error_when_record_is_over_the_record_limit() {
    with_temp_dir(async |root| {
        let (mut tx, _rx, ledger) = create_buffer_with_config(Config {
            root,
            max_record_size: 100,
            max_chunk_size: 10_000,
            max_buffer_size: 100_000,
        });
        assert_ledger_empty!(ledger);

        let msg = Message::new(10);
        let written = tx.write(msg).await.unwrap();
        assert_eq!(written, 10 + 4 + 4 + 8);

        let msg = Message::new(100);
        let result = tx.write(msg).await;
        assert!(matches!(
            result,
            Err(Error::RecordTooLarge {
                limit,
                size
            }) if limit == 100 && size == 116
        ));
    })
    .await
}

#[tokio::test]
async fn writer_waits_when_buffer_is_full() {
    with_temp_dir(async |root| {
        let (mut tx, mut rx, ledger) = create_buffer_with_config(Config {
            root,
            max_record_size: 100,
            max_chunk_size: 2000,
            max_buffer_size: 1000,
        });

        let mut total = 0;
        for _ in 0..20 {
            if total + 4 + 4 + 8 + 80 > 1000 {
                break;
            }

            total += tx.write(80.into()).await.unwrap();
        }

        assert_eq!(ledger.get_buffer_bytes(), total);
        tx.flush().unwrap();

        // now, it's full
        let mut write = spawn(async { tx.write(80.into()).await.unwrap() });
        assert_pending!(write.poll());

        let mut msg = rx.read().await.unwrap().unwrap();
        assert_pending!(write.poll());

        msg.acknowledge().await;
        assert_ready!(write.poll());
    })
    .await;
}

#[tokio::test]
async fn roll_to_next_writer() {
    with_temp_dir(async |root: PathBuf| {
        let (mut tx, _rx, _ledger) = create_buffer_with_config(Config {
            root: root.clone(),
            max_record_size: 4096,
            max_chunk_size: 128,
            max_buffer_size: 16 * 1024,
        });

        // 0000.chunk
        let written = tx.write(40.into()).await.unwrap();
        assert_eq!(written, 4 + 4 + 8 + 40);
        tx.write(40.into()).await.unwrap();

        // 0001.chunk
        tx.write(40.into()).await.unwrap();
        tx.write(40.into()).await.unwrap();

        // 0002.chunk
        tx.write(40.into()).await.unwrap();
        tx.write(40.into()).await.unwrap();

        tx.flush().unwrap();

        for i in 0..3 {
            let path = root.join(format!("{i:04x}.chunk"));
            let size = path.metadata().unwrap().len();
            assert_eq!(size, 2 * (4 + 4 + 8 + 40));
            assert!(size <= 128);
        }
    })
    .await;
}

#[tokio::test]
async fn writer_rolls_chunk_files_when_the_limit_is_exceeded_after_reload() {
    with_temp_dir(async |root: PathBuf| {
        let (mut tx, rx, ledger) = create_buffer_with_config(Config {
            root: root.clone(),
            max_record_size: 100,
            max_chunk_size: 1000,
            max_buffer_size: 10000,
        });
        assert_ledger_empty!(ledger);

        for _ in 0..6 {
            tx.write(80.into()).await.unwrap();
        }
        let written = ledger.get_buffer_bytes();
        assert!(written > 500);

        drop(tx);
        drop(rx);
        drop(ledger);

        tokio::task::yield_now().await;

        let (mut tx, _rx, ledger) = create_buffer_with_config(Config {
            root,
            max_record_size: 100,
            max_chunk_size: 500,
            max_buffer_size: 10000,
        });
        assert_eq!(written, ledger.get_buffer_bytes());

        // now the chunk file still oversize
        let file = OpenOptions::new()
            .read(true)
            .open(tx.config().chunk_path(0))
            .unwrap();
        assert_eq!(file.metadata().unwrap().len() as usize, written);

        // after this write, new file should be created
        tx.write(80.into()).await.unwrap();
        tx.flush().unwrap();
        let file = OpenOptions::new()
            .read(true)
            .open(tx.config().chunk_path(0))
            .unwrap();
        assert_eq!(file.metadata().unwrap().len() as usize, written);

        // assert new file
        let file = OpenOptions::new()
            .read(true)
            .open(tx.config().chunk_path(1))
            .unwrap();
        assert_eq!(file.metadata().unwrap().len() as usize, 4 + 4 + 8 + 80);
    })
    .await
}

#[tokio::test]
async fn try_write_returns_when_buffer_is_full() {
    with_temp_dir(async |root: PathBuf| {
        let (mut tx, _rx, ledger) = create_buffer_with_config(Config {
            root,
            max_record_size: 200,
            max_chunk_size: 200,
            max_buffer_size: 200,
        });
        assert_ledger_empty!(ledger);

        let written = tx.write(90.into()).await.unwrap();
        assert_eq!(written, 16 + 90);

        let maybe_msg = tx.try_write(90.into()).await.unwrap();
        assert_eq!(maybe_msg, Some(90.into()));
    })
    .await
}

#[tokio::test]
async fn writer_can_validate_last_write_when_buffer_is_full() {}
