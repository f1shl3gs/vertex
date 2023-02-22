use event::{BatchNotifier, Event, Events, LogRecord};
use futures::{stream, Stream, StreamExt};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::collections::HashMap;
use std::iter;

pub fn random_string(len: usize) -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect::<String>()
}

pub fn random_lines(len: usize) -> impl Iterator<Item = String> {
    std::iter::repeat(()).map(move |_| random_string(len))
}

pub fn random_map(max_size: usize, field_len: usize) -> HashMap<String, String> {
    let size = thread_rng().gen_range(0..max_size);

    (0..size)
        .map(move |_| (random_string(field_len), random_string(field_len)))
        .collect()
}

pub fn random_maps(
    max_size: usize,
    field_len: usize,
) -> impl Iterator<Item = HashMap<String, String>> {
    iter::repeat(()).map(move |_| random_map(max_size, field_len))
}

pub fn random_lines_with_stream(
    len: usize,
    count: usize,
    batch: Option<BatchNotifier>,
) -> (Vec<String>, impl Stream<Item = Events>) {
    let generator = move |_| random_string(len);
    generate_lines_with_stream(generator, count, batch)
}

pub fn generate_lines_with_stream<Gen: FnMut(usize) -> String>(
    generator: Gen,
    count: usize,
    batch: Option<BatchNotifier>,
) -> (Vec<String>, impl Stream<Item = Events>) {
    let lines = (0..count).map(generator).collect::<Vec<_>>();
    let stream = map_batch_stream(stream::iter(lines.clone()).map(LogRecord::from), batch);
    (lines, stream)
}

// TODO refactor to have a single implementation for `Event`, `LogRecord` and `Metric`.
fn map_batch_stream(
    stream: impl Stream<Item = LogRecord>,
    batch: Option<BatchNotifier>,
) -> impl Stream<Item = Events> {
    stream.map(move |log| vec![log.with_batch_notifier_option(&batch)].into())
}

pub fn generate_events_with_stream<Gen: FnMut(usize) -> Event>(
    generator: Gen,
    count: usize,
    batch: Option<BatchNotifier>,
) -> (Vec<Event>, impl Stream<Item = Events>) {
    let events = (0..count).map(generator).collect::<Vec<_>>();
    let stream = map_batch_stream(
        stream::iter(events.clone()).map(|event| event.into_log()),
        batch,
    );
    (events, stream)
}

pub fn random_events_with_stream(
    len: usize,
    count: usize,
    batch: Option<BatchNotifier>,
) -> (Vec<Event>, impl Stream<Item = Events>) {
    let events = (0..count)
        .map(|_| Event::from(random_string(len)))
        .collect::<Vec<_>>();

    let stream = map_batch_stream(
        stream::iter(events.clone()).map(|event| event.into_log()),
        batch,
    );

    (events, stream)
}
