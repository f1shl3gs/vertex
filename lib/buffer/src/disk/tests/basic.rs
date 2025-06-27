use super::{create_buffer, with_temp_dir};
use crate::assert_ledger_empty;
use crate::tests::Message;

#[tokio::test]
async fn read_and_write() {
    with_temp_dir(async |root| {
        let (mut writer, mut reader, ledger) = create_buffer(root);
        assert_ledger_empty!(ledger);

        let want = (20..30)
            .cycle()
            .take(10)
            .map(Message::new)
            .collect::<Vec<_>>();

        let input = want.clone();
        let write_task = tokio::spawn(async move {
            for msg in input {
                writer.write(msg).await.unwrap();
            }

            writer.flush().unwrap();

            // reader will be notified, if writer dropped
        });

        let read_task = tokio::spawn(async move {
            let mut got = Vec::new();
            while let Some(mut msg) = reader.read().await.unwrap() {
                msg.acknowledge().await;
                got.push(msg);
            }

            got
        });

        // Wait for both tasks to complete
        write_task.await.unwrap();

        let got = read_task.await.unwrap();
        assert_eq!(want, got);

        ledger.flush().unwrap();
        assert_ledger_empty!(ledger);
    })
    .await;
}

#[tokio::test]
async fn reader_exits_cleanly_when_writer_done_and_inflight_acknowledgements() {
    with_temp_dir(async |root| {
        let (mut writer, mut reader, ledger) = create_buffer(root);
        assert_ledger_empty!(ledger);

        // Now write a single value and close the writer
        let msg = Message::new(32);
        writer.write(msg).await.unwrap();
        writer.flush().unwrap();
        drop(writer);
        println!("{ledger:#?}");
        assert_eq!(ledger.get_next_write_record_id(), 2);

        // And read that single value
        let mut first_read = reader.read().await.unwrap().unwrap();
        assert_eq!(ledger.get_buffer_records(), 1);
        first_read.acknowledge().await;
        assert_eq!(ledger.get_buffer_records(), 0);
        assert_eq!(first_read.size, 32);

        // writer is closed already, so the None is returned.
        let second_read = reader.read().await.unwrap();
        assert!(second_read.is_none());
    })
    .await;
}
