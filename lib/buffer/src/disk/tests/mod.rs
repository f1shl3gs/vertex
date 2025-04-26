#![allow(clippy::print_stdout)]

mod acknowledgements;
mod basic;
mod initialization;
mod invariants;
mod known_errors;
mod record;
mod size_limit;

use std::path::PathBuf;
use std::sync::Arc;

use rand::Rng;
use rand::distr::Alphanumeric;
use tracing::trace;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::util::SubscriberInitExt;

use crate::disk::{Config, Ledger, Reader, Writer};
use crate::tests::Message;

async fn with_temp_dir<F>(f: F)
where
    F: AsyncFnOnce(PathBuf) -> (),
{
    let _ = tracing_subscriber::fmt::SubscriberBuilder::default()
        .with_env_filter(EnvFilter::from_default_env())
        // .with_span_events(FmtSpan::ENTER)
        .finish()
        .try_init(); // unwrap cannot be handled here, because tests might be run parallelly

    let mut rng = rand::rng();
    let dir = (&mut rng)
        .sample_iter(Alphanumeric)
        .take(8)
        .map(char::from)
        .collect::<String>();

    let path = std::env::temp_dir().join("buffer-tests").join(dir);
    std::fs::create_dir_all(&path).unwrap();

    trace!(message = "temp directory created", ?path);

    f(path).await;
}

fn create_buffer(root: PathBuf) -> (Writer<Message>, Reader<Message>, Arc<Ledger>) {
    create_buffer_with_config(Config {
        root,
        max_record_size: 4 * 1024,
        max_chunk_size: 16 * 1024,
        max_buffer_size: 64 * 1024,
    })
}

fn create_buffer_with_config(config: Config) -> (Writer<Message>, Reader<Message>, Arc<Ledger>) {
    let (writer, reader) = config.build::<Message>().unwrap();
    let ledger = writer.ledger();

    (writer, reader, ledger)
}

#[macro_export]
macro_rules! assert_ledger_empty {
    ($ledger:expr) => {
        assert_eq!(
            $ledger.get_buffer_records(),
            0,
            "ledger should have 0 records, but had {}",
            $ledger.get_buffer_records()
        );
        assert_eq!(
            $ledger.get_buffer_bytes(),
            0,
            "ledger should have 0 bytes, but had {} bytes",
            $ledger.get_buffer_bytes()
        )
    };
}
