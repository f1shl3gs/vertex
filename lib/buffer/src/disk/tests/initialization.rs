use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

use crate::assert_ledger_empty;
use crate::disk::Config;
use crate::disk::tests::{create_buffer, create_buffer_with_config, with_temp_dir};

#[tokio::test]
async fn reader_ignore_partial_write_on_last_record() {
    with_temp_dir(async |root| {
        let (mut writer, mut reader, ledger) = create_buffer(root.clone());
        assert_ledger_empty!(ledger);

        let written = writer.write(64.into()).await.unwrap();
        writer.flush().unwrap();
        drop(writer);

        let mut msg = reader.read().await.unwrap().unwrap();
        assert_eq!(ledger.get_buffer_records(), 1);
        assert_eq!(ledger.get_buffer_bytes(), written);
        msg.acknowledge().await;

        let second_read = reader.read().await.unwrap();
        assert!(second_read.is_none());
        assert_ledger_empty!(ledger);

        ledger.flush().unwrap();

        drop(reader);
        drop(ledger);

        tokio::task::yield_now().await;

        let mut file = OpenOptions::new()
            .write(true)
            .open(root.join("0000.chunk"))
            .unwrap();
        file.set_len(64).unwrap();
        file.flush().unwrap();
        file.sync_all().unwrap();
        drop(file);

        let (mut writer, mut reader, ledger) = create_buffer(root);
        assert_ledger_empty!(ledger);

        writer.write(64.into()).await.unwrap();

        let mut msg = reader.read().await.unwrap().unwrap();
        msg.acknowledge().await;

        assert_ledger_empty!(ledger);
    })
    .await;
}

#[tokio::test]
async fn load_ledger_correctly() {
    with_temp_dir(async |root: PathBuf| {
        let (mut tx, rx, ledger) = create_buffer_with_config(Config {
            root: root.clone(),
            max_record_size: 4096,
            max_chunk_size: 128,
            max_buffer_size: 16 * 1024,
        });
        assert_ledger_empty!(ledger);

        // 0000.chunk
        let written = tx.write(40.into()).await.unwrap();
        assert_eq!(written, 4 + 4 + 8 + 40);
        tx.write(40.into()).await.unwrap();

        assert_eq!(ledger.get_last_read_record_id(), 0);
        assert_eq!(ledger.get_next_write_record_id(), 3);

        // 0001.chunk
        tx.write(40.into()).await.unwrap();
        tx.write(40.into()).await.unwrap();

        // 0002.chunk
        tx.write(40.into()).await.unwrap();
        tx.write(40.into()).await.unwrap();

        assert_eq!(ledger.get_buffer_records(), 6);

        tx.flush().unwrap();
        ledger.flush().unwrap();
        drop(tx);
        drop(rx);
        drop(ledger);

        tokio::task::yield_now().await;

        let (_tx, _rx, ledger) = create_buffer_with_config(Config {
            root,
            max_record_size: 4096,
            max_chunk_size: 128,
            max_buffer_size: 16 * 1024,
        });

        assert_eq!(ledger.get_buffer_records(), 6);
        assert_eq!(ledger.get_buffer_bytes(), written * 6);
    })
    .await;
}
