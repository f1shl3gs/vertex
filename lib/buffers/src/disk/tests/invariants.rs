use tokio_test::{assert_pending, task::spawn};
use tracing::Instrument;

use super::{
    create_buffer_with_max_data_file_size, install_tracing_helpers, with_temp_dir, SizedRecord,
};
use crate::{
    assert_buffer_is_empty, assert_buffer_records, assert_buffer_size, assert_enough_bytes_written,
    assert_reader_writer_file_positions,
    disk_v2::{common::MAX_FILE_ID, tests::create_default_buffer},
};

#[tokio::test]
async fn last_record_is_valid_during_load_when_buffer_correctly_flushed_and_stopped() {
    let assertion_registry = install_tracing_helpers();

    let fut = with_temp_dir(|dir| {
        let data_dir = dir.to_path_buf();

        async move {
            let reset_called = assertion_registry
                .build()
                .with_name("reset")
                .with_parent_name(
                    "last_record_is_valid_during_load_when_buffer_correctly_flushed_and_stopped",
                )
                .was_entered()
                .finalize();

            // Create a normal buffer.
            let (mut writer, _, _, ledger) = create_default_buffer(data_dir.clone()).await;
            let bytes_written = writer
                .write_record(SizedRecord(64))
                .await
                .expect("write should not fail");
            assert_enough_bytes_written!(bytes_written, SizedRecord, 64);

            writer.flush().await.expect("flush should not fail");
            ledger.flush().expect("flush should not fail");

            drop(writer);
            drop(ledger);

            // Make sure we can open the buffer again without any errors.
            let (_, _, _, ledger) = create_default_buffer::<_, SizedRecord>(data_dir).await;
            assert_eq!(ledger.get_total_records(), 1);

            // TODO: Rewrite this as a negative assertion (i.e. `was_not_entered`) once
            // `tracing-fluent-assertions` adds support for them.
            assert!(!reset_called.try_assert());
        }
    });

    let parent =
        trace_span!("last_record_is_valid_during_load_when_buffer_correctly_flushed_and_stopped");
    fut.instrument(parent).await;
}

#[tokio::test]
async fn file_id_wraps_around_when_max_file_id_hit() {
    with_temp_dir(|dir| {
        let data_dir = dir.to_path_buf();

        async move {
            let record_size: u32 = 100;

            // Create our buffer with an arbitrarily low max data file size, which will let us
            // quickly run through the file ID range.
            let (mut writer, mut reader, acker, ledger) =
                create_buffer_with_max_data_file_size(data_dir, u64::from(record_size)).await;

            assert_buffer_is_empty!(ledger);
            assert_reader_writer_file_positions!(ledger, 0, 0);

            // We execute a loop of writing and then reading back a record, and we assert each time
            // that the file IDs are where we expect them to be.  We write 3x the number of records
            // as the max possible file ID, to ensure that rollover works.
            let file_id_upper = MAX_FILE_ID;

            // Random addition at the end so we don't land explicitly on the u16 boundary.
            let target_id = (u32::from(file_id_upper) * 3) + 15;

            let mut id = 0;
            let mut reader_file_id = 0;
            let mut writer_file_id = 0;
            while id < target_id {
                let record = SizedRecord(record_size);
                let bytes_written = writer
                    .write_record(record.clone())
                    .await
                    .expect("write should not fail");
                assert_enough_bytes_written!(bytes_written, SizedRecord, record_size);

                writer.flush().await.expect("flush should not fail");

                let record_read = reader.next().await.expect("read should not fail");
                assert_eq!(record_read, Some(record));

                acker.ack(1);

                let expected_file_id = u16::try_from(id % u32::from(file_id_upper))
                    .expect("should never be greater than u16");
                let (actual_reader_file_id, actual_writer_file_id) =
                    ledger.get_current_reader_writer_file_id();
                reader_file_id = actual_reader_file_id;
                writer_file_id = actual_writer_file_id;

                // Record count/total size will always match the write we just did because
                // acknowledgement is only driven by calls to `next`, but our reader/writer should
                // be in lockstep, since no data files are closed/adjusted before a read/write
                // complete, only once we attempt the next one.
                assert_eq!(reader_file_id, writer_file_id);
                assert_eq!(expected_file_id, reader_file_id);
                assert_eq!(expected_file_id, writer_file_id);
                assert_buffer_size!(ledger, 1, bytes_written);

                id += 1;
            }

            writer.close();

            // After closing the writer, our final read should tell us that the buffer is closed,
            // but as important, it should tell us that the reader/writer file IDs haven't changed
            // since we left the loop _and_ that they're still in lockstep.
            let final_read = reader.next().await.expect("read should not fail");
            assert_eq!(final_read, None);
            assert_buffer_is_empty!(ledger);
            assert_reader_writer_file_positions!(ledger, reader_file_id, writer_file_id);
        }
    })
    .await;
}

#[tokio::test]
async fn writer_stops_when_hitting_file_that_reader_is_still_on() {
    let assertion_registry = install_tracing_helpers();

    let fut = with_temp_dir(|dir| {
        let data_dir = dir.to_path_buf();

        async move {
            let record_size: u32 = 100;

            // Create our buffer with an arbitrarily low max data file size, which will let us
            // quickly run through the file ID range.
            let (mut writer, mut reader, acker, ledger) =
                create_buffer_with_max_data_file_size(data_dir, u64::from(record_size)).await;

            assert_buffer_is_empty!(ledger);
            assert_reader_writer_file_positions!(ledger, 0, 0);

            // We execute a loop of writing enough records to consume all data files, without doing
            // any reading.
            let file_id_upper = u32::from(MAX_FILE_ID);

            let mut id = 0;
            let mut total_size = 0;
            while id < file_id_upper {
                let record = SizedRecord(record_size);
                let bytes_written = writer
                    .write_record(record)
                    .await
                    .expect("write should not fail");
                assert_enough_bytes_written!(bytes_written, SizedRecord, record_size);

                writer.flush().await.expect("flush should not fail");

                assert_reader_writer_file_positions!(ledger, 0, id);

                id += 1;
                total_size += bytes_written;
            }

            assert_buffer_size!(ledger, 32, total_size);
            assert_reader_writer_file_positions!(ledger, 0, 31);

            let assertion = assertion_registry
                .build()
                .with_name("wait_for_reader")
                .with_parent_name("writer_stops_when_hitting_file_that_reader_is_still_on")
                .was_entered()
                .finalize();

            // Now we should be consuming all data files, and our next write should block trying to
            // open the "first" data file until we do a read.
            let mut blocked_write = spawn(async {
                let record = SizedRecord(record_size);
                writer.write_record(record).await
            });

            // You might be looking at the assert_pending! calls below and wondering what's
            // happening there.  Essentially, the process of doing a read or write could contain a
            // variable number of asynchronous steps required to open the data file, or wait for the
            // reader/writer to make progress, and so on.  Since we're executing real file I/O
            // operations in these tests, these things aren't deterministic.
            //
            // Rather than transform all of the code so it can be fully mocked and controlled, we've
            // opted for a lightweight approach where we assert conditions around tracing spans, in
            // the sense of asserting that certain spans have been entered, and so on.
            //
            // We're trying to make sure our code gets to the point of waiting for the reader to
            // wake up, which would imply that a reader needs to issue a wake-up for progress to be
            // made.  We create an assertion that looks for that, and we fallibly assert it in a
            // loop while polling the blocked write to drive it forward.  Once that assertion
            // becomes true, we know the blocked write is now waiting on the reader.
            //
            // There might still be spurious wakeups from some of the other asynchronous code in the
            // call, but our blocked write will _not_ proceed until the reader itself specifically
            // wakes it up, which is all that matters for our logic.
            while !assertion.try_assert() {
                assert_pending!(blocked_write.poll());
            }
            assert_pending!(blocked_write.poll());

            // Now execute a read which will pull the first record.  This doesn't yet delete the
            // first data file since we haven't acknowledged the read yet, so the file can't yet be
            // deleted.
            let first_record_read = reader.next().await.expect("read should not fail");
            assert_eq!(first_record_read, Some(SizedRecord(record_size)));
            assert_buffer_size!(ledger, 32, total_size);
            assert_reader_writer_file_positions!(ledger, 0, 31);

            acker.ack(1);

            // Our write should still not yet be ready because we won't have acknowledged the
            // read until we call `next` one more time, which will not only acknowledge the write,
            // driving a wake-up, but will queue the first data file to be deleted once it
            // recognizes the first data file is complete, and before loading the next data file, it
            // should also delete the first data file:
            assert_pending!(blocked_write.poll());

            let second_record_read = reader.next().await.expect("read should not fail");
            assert_eq!(second_record_read, Some(SizedRecord(record_size)));
            assert_buffer_records!(ledger, 31);
            assert_reader_writer_file_positions!(ledger, 1, 31);

            // Now our writer should be woken up as we deleted the first data file when we went
            // through the second read, which triggers a writer wake-up.  We await the future
            // directly because the writer is going to go through a few blocking file operations as
            // it flushes the old file and opens the new one, and this means the very next poll
            // won't actually return immediately, so we just await instead of looping or anything:
            let bytes_written = blocked_write.await.expect("write should not fail");
            assert_enough_bytes_written!(bytes_written, SizedRecord, record_size);

            // Technically, we'll have 32 records in flight at this point, despite two reads,
            // because again, we haven't acknowledged the second read, so the record is still
            // considered to be outstanding.  We should, however, have moved on to our next data
            // file in the writer:
            assert_buffer_records!(ledger, 32);
            assert_reader_writer_file_positions!(ledger, 1, 0);
        }
    });

    let parent = trace_span!("writer_stops_when_hitting_file_that_reader_is_still_on");
    fut.instrument(parent).await;
}

#[tokio::test]
async fn reader_still_works_when_record_id_wraps_around() {
    with_temp_dir(|dir| {
        let data_dir = dir.to_path_buf();

        async move {
            // Create a simple buffer.
            let (_, _, _, ledger) = create_default_buffer::<_, SizedRecord>(data_dir.clone()).await;

            assert_buffer_is_empty!(ledger);
            assert_reader_writer_file_positions!(ledger, 0, 0);

            // Adjust the record IDs manually so they comes right before the rollover event.
            //
            // We have to adjust both the writer and reader record ID markers.
            unsafe {
                ledger.state().unsafe_set_writer_next_record_id(u64::MAX);
            }
            unsafe {
                ledger
                    .state()
                    .unsafe_set_reader_last_record_id(u64::MAX - 1);
            }

            ledger.flush().expect("ledger should not fail to flush");
            assert_eq!(u64::MAX, ledger.state().get_next_writer_record_id());
            assert_eq!(u64::MAX - 1, ledger.state().get_last_reader_record_id());

            // We know that the reader will get angry when it goes to read a record, because at
            // startup it determined that the next record it reads should have a record ID of 1.
            //
            // The final step to get our ledger into the correct state is to simply reload the
            // buffer entirely, so the reader and writer initialize themselves with the ledger
            // stating that we're close to having written 2^64 records already.
            drop(ledger);

            let (mut writer, mut reader, acker, ledger) = create_default_buffer(data_dir).await;

            // Now we do two writes: one which uses u64::MAX, and another which will get the rolled
            // over value and go back to 0.
            let first_record_size = 14;
            let first_bytes_written = writer
                .write_record(SizedRecord(first_record_size))
                .await
                .expect("write should not fail");
            assert_enough_bytes_written!(first_bytes_written, SizedRecord, first_record_size);
            assert_eq!(0, ledger.state().get_next_writer_record_id());

            writer.flush().await.expect("flush should not fail");
            assert_buffer_records!(ledger, 1);

            let second_record_size = 256;
            let second_bytes_written = writer
                .write_record(SizedRecord(second_record_size))
                .await
                .expect("write should not fail");
            assert_enough_bytes_written!(second_bytes_written, SizedRecord, second_record_size);
            assert_eq!(1, ledger.state().get_next_writer_record_id());

            writer.flush().await.expect("flush should not fail");
            assert_buffer_records!(ledger, 2);

            writer.close();

            // Now we should be able to read both records without the reader getting angry.
            let first_record_read = reader.next().await.expect("read should not fail");
            assert_eq!(first_record_read, Some(SizedRecord(first_record_size)));
            assert_eq!(u64::MAX - 1, ledger.state().get_last_reader_record_id());
            assert_buffer_records!(ledger, 2);

            acker.ack(1);

            let second_record_read = reader.next().await.expect("read should not fail");
            assert_eq!(second_record_read, Some(SizedRecord(second_record_size)));
            assert_eq!(u64::MAX, ledger.state().get_last_reader_record_id());
            assert_buffer_records!(ledger, 1);

            acker.ack(1);

            let final_read = reader.next().await.expect("read should not fail");
            assert_eq!(final_read, None);
            assert_eq!(0, ledger.state().get_last_reader_record_id());
            assert_buffer_is_empty!(ledger);
        }
    })
    .await;
}

#[allow(clippy::too_many_lines)]
#[tokio::test]
async fn reader_deletes_data_file_around_record_id_wraparound() {
    let assertion_registry = install_tracing_helpers();
    let fut = with_temp_dir(|dir| {
        let data_dir = dir.to_path_buf();

        async move {
            // Create a simple buffer.
            let (_, _, _, ledger) = create_default_buffer::<_, SizedRecord>(data_dir.clone()).await;

            assert_buffer_is_empty!(ledger);
            assert_reader_writer_file_positions!(ledger, 0, 0);

            // Adjust the record IDs manually so they comes right before the rollover event.
            //
            // We have to adjust both the writer and reader record ID markers.
            unsafe {
                ledger.state().unsafe_set_writer_next_record_id(u64::MAX);
            }
            unsafe {
                ledger
                    .state()
                    .unsafe_set_reader_last_record_id(u64::MAX - 1);
            }

            ledger.flush().expect("ledger should not fail to flush");
            assert_eq!(u64::MAX, ledger.state().get_next_writer_record_id());
            assert_eq!(u64::MAX - 1, ledger.state().get_last_reader_record_id());

            // We know that the reader will get angry when it goes to read a record, because at
            // startup it determined that the next record it reads should have a record ID of 1.
            //
            // The final step to get our ledger into the correct state is to simply reload the
            // buffer entirely, so the reader and writer initialize themselves with the ledger
            // stating that we're close to having written 2^64 records already.
            drop(ledger);

            let (mut writer, mut reader, acker, ledger) =
                create_buffer_with_max_data_file_size(data_dir, 256).await;
            let starting_writer_file_id = ledger.get_current_writer_file_id();
            let next_writer_file_id = ledger.get_next_writer_file_id();

            // Now we do three writes, which will have u64::MAX, 0, and 1 as the IDs.  This ensures
            // that the data file will have at least three records, and a range of IDs that cross
            // the wrapping threshold.
            let first_record_size = 80;
            let first_bytes_written = writer
                .write_record(SizedRecord(first_record_size))
                .await
                .expect("write should not fail");
            assert_enough_bytes_written!(first_bytes_written, SizedRecord, first_record_size);
            assert_eq!(0, ledger.state().get_next_writer_record_id());

            let second_record_size = 82;
            let second_bytes_written = writer
                .write_record(SizedRecord(second_record_size))
                .await
                .expect("write should not fail");
            assert_enough_bytes_written!(second_bytes_written, SizedRecord, second_record_size);
            assert_eq!(1, ledger.state().get_next_writer_record_id());

            let third_record_size = 84;
            let third_bytes_written = writer
                .write_record(SizedRecord(third_record_size))
                .await
                .expect("write should not fail");
            assert_enough_bytes_written!(third_bytes_written, SizedRecord, third_record_size);
            assert_eq!(2, ledger.state().get_next_writer_record_id());

            writer.flush().await.expect("flush should not fail");
            assert_buffer_records!(ledger, 3);
            assert_reader_writer_file_positions!(ledger, 0, starting_writer_file_id);

            // Now our third write should have exceeded the data file, so this next write should hit
            // a new data file:
            let fourth_record_size = 86;
            let fourth_bytes_written = writer
                .write_record(SizedRecord(fourth_record_size))
                .await
                .expect("write should not fail");
            assert_enough_bytes_written!(fourth_bytes_written, SizedRecord, fourth_record_size);
            assert_eq!(3, ledger.state().get_next_writer_record_id());

            writer.flush().await.expect("flush should not fail");
            assert_buffer_records!(ledger, 4);
            assert_reader_writer_file_positions!(ledger, 0, next_writer_file_id);

            writer.close();

            // Now that we have our two data files, we want to read all the records back,
            // acknowledge them, and assert that we deleted the first data file:
            let deleted_data_file = assertion_registry
                .build()
                .with_name("delete_completed_data_file")
                .with_parent_name("reader_deletes_data_file_around_record_id_wraparound")
                .was_entered()
                .finalize();

            let first_record_read = reader.next().await.expect("read should not fail");
            assert_eq!(first_record_read, Some(SizedRecord(first_record_size)));
            assert_eq!(u64::MAX - 1, ledger.state().get_last_reader_record_id());
            assert_buffer_records!(ledger, 4);
            assert_reader_writer_file_positions!(ledger, 0, next_writer_file_id);

            acker.ack(1);
            assert!(!deleted_data_file.try_assert());

            let second_record_read = reader.next().await.expect("read should not fail");
            assert_eq!(second_record_read, Some(SizedRecord(second_record_size)));
            assert_eq!(u64::MAX, ledger.state().get_last_reader_record_id());
            assert_buffer_records!(ledger, 3);
            assert_reader_writer_file_positions!(ledger, 0, next_writer_file_id);

            acker.ack(1);
            assert!(!deleted_data_file.try_assert());

            let third_record_read = reader.next().await.expect("read should not fail");
            assert_eq!(third_record_read, Some(SizedRecord(third_record_size)));
            assert_eq!(0, ledger.state().get_last_reader_record_id());
            assert_buffer_records!(ledger, 2);
            assert_reader_writer_file_positions!(ledger, 0, next_writer_file_id);

            acker.ack(1);
            assert!(!deleted_data_file.try_assert());

            // This read should be where we actually delete the file since we've acknowledged all of
            // the reads from the first data file:
            let fourth_record_read = reader.next().await.expect("read should not fail");
            assert_eq!(fourth_record_read, Some(SizedRecord(fourth_record_size)));
            assert_eq!(1, ledger.state().get_last_reader_record_id());
            assert_buffer_records!(ledger, 1);
            assert_reader_writer_file_positions!(ledger, 1, next_writer_file_id);
            assert!(deleted_data_file.try_assert());

            acker.ack(1);

            // And now since we closed the writer and read all four records, we should be done:
            let final_read = reader.next().await.expect("read should not fail");
            assert_eq!(final_read, None);
            assert_eq!(2, ledger.state().get_last_reader_record_id());
            assert_reader_writer_file_positions!(ledger, 1, next_writer_file_id);
            assert_buffer_is_empty!(ledger);
        }
    });

    let parent = trace_span!("reader_deletes_data_file_around_record_id_wraparound");
    fut.instrument(parent).await;
}
