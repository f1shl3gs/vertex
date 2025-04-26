use super::{create_buffer, with_temp_dir};

#[tokio::test]
async fn acknowledgement_update_ledger_correctly() {
    with_temp_dir(async |root| {
        let (mut writer, mut reader, ledger) = create_buffer(root);
        assert_eq!(ledger.state().get_last_read_record_id(), 0);
        assert_eq!(ledger.state().get_next_write_record_id(), 1);
        assert_eq!(ledger.state().get_current_reader_file_id(), 0);
        assert_eq!(ledger.state().get_current_writer_file_id(), 0);

        writer.write(10.into()).await.unwrap();
        assert_eq!(ledger.state().get_last_read_record_id(), 0);
        assert_eq!(ledger.state().get_next_write_record_id(), 2);
        assert_eq!(ledger.state().get_current_reader_file_id(), 0);
        assert_eq!(ledger.state().get_current_writer_file_id(), 0);

        let mut msg = reader.read().await.unwrap().unwrap();
        assert_eq!(ledger.state().get_last_read_record_id(), 0);
        assert_eq!(ledger.state().get_next_write_record_id(), 2);
        assert_eq!(ledger.state().get_current_reader_file_id(), 0);
        assert_eq!(ledger.state().get_current_writer_file_id(), 0);

        msg.acknowledge().await;
        assert_eq!(ledger.state().get_last_read_record_id(), 1);
        assert_eq!(ledger.state().get_next_write_record_id(), 2);
        assert_eq!(ledger.state().get_current_reader_file_id(), 0);
        assert_eq!(ledger.state().get_current_writer_file_id(), 0);
    })
    .await
}

#[tokio::test]
async fn reload_correct() {
    with_temp_dir(async |root| {
        let (mut writer, mut reader, ledger) = create_buffer(root.clone());
        assert_eq!(ledger.state().get_last_read_record_id(), 0);
        assert_eq!(ledger.state().get_next_write_record_id(), 1);
        assert_eq!(ledger.state().get_current_reader_file_id(), 0);
        assert_eq!(ledger.state().get_current_writer_file_id(), 0);

        writer.write(10.into()).await.unwrap();
        assert_eq!(ledger.state().get_last_read_record_id(), 0);
        assert_eq!(ledger.state().get_next_write_record_id(), 2);
        assert_eq!(ledger.state().get_current_reader_file_id(), 0);
        assert_eq!(ledger.state().get_current_writer_file_id(), 0);

        let mut msg = reader.read().await.unwrap().unwrap();
        assert_eq!(ledger.state().get_last_read_record_id(), 0);
        assert_eq!(ledger.state().get_next_write_record_id(), 2);
        assert_eq!(ledger.state().get_current_reader_file_id(), 0);
        assert_eq!(ledger.state().get_current_writer_file_id(), 0);

        msg.acknowledge().await;
        assert_eq!(ledger.state().get_last_read_record_id(), 1);
        assert_eq!(ledger.state().get_next_write_record_id(), 2);
        assert_eq!(ledger.state().get_current_reader_file_id(), 0);
        assert_eq!(ledger.state().get_current_writer_file_id(), 0);

        drop(writer);
        drop(reader);
        drop(ledger);
        // yield to make sure finalizer exit
        tokio::task::yield_now().await;

        let (_writer, _reader, ledger) = create_buffer(root);
        assert_eq!(ledger.state().get_last_read_record_id(), 1);
        assert_eq!(ledger.state().get_next_write_record_id(), 2);
        assert_eq!(ledger.state().get_current_reader_file_id(), 0);
        assert_eq!(ledger.state().get_current_writer_file_id(), 0);
    })
    .await;
}
