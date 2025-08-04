use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Duration;

use futures::{FutureExt, StreamExt};
use tail::decode::NewlineDecoder;
use tail::{Checkpointer, Conveyor, FileReader, Provider, ReadFrom, Shutdown, harvest};
use tokio_util::codec::FramedRead;

struct StaticProvider {
    paths: Vec<PathBuf>,
}

impl Provider for StaticProvider {
    type Metadata = PathBuf;

    async fn scan(&mut self) -> std::io::Result<Vec<(PathBuf, PathBuf)>> {
        // sleep a while to avoid this function burning the CPU
        tokio::time::sleep(Duration::from_millis(500)).await;

        let paths = self
            .paths
            .iter()
            .map(|path| (path.clone(), path.clone()))
            .collect::<Vec<_>>();

        Ok(paths)
    }
}

#[derive(Clone, Default)]
struct TrackedOutput {
    consumed: Arc<AtomicUsize>,
}

impl Conveyor for TrackedOutput {
    type Metadata = PathBuf;

    fn run(
        &self,
        reader: FileReader,
        _meta: Self::Metadata,
        _offset: Arc<AtomicU64>,
        _shutdown: Shutdown,
    ) -> impl Future<Output = Result<(), ()>> + Send + 'static {
        let mut reader = FramedRead::new(reader, NewlineDecoder::new(4 * 1024));
        let consumed = Arc::clone(&self.consumed);

        Box::pin(async move {
            while let Some(Ok((_data, size))) = reader.next().await {
                consumed.fetch_add(size, Ordering::Relaxed);
            }

            Ok(())
        })
    }
}

// This example is used to test this library, so the performance does not matter,
// and if something blocking the thread, we can notice it immediately.
#[tokio::main(flavor = "current_thread")]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .init();

    let files = std::env::args().skip(1).collect::<Vec<_>>();
    let provider = StaticProvider {
        paths: files.into_iter().map(PathBuf::from).collect::<Vec<_>>(),
    };

    let consumed = Arc::new(AtomicUsize::new(0));
    let conveyor = TrackedOutput {
        consumed: Arc::clone(&consumed),
    };

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        let mut last = 0;

        loop {
            interval.tick().await;

            let current = consumed.load(Ordering::Acquire);
            if current == last {
                continue;
            }

            let rate = (current - last) as f64 / 1024.0 / 1024.0;
            last = current;
            println!("consumed: {current:12}, rate: {rate:>16.4} M/s");
        }
    });

    let root = std::env::current_dir().unwrap();
    let checkpointer = Checkpointer::load(root).unwrap();

    let (trigger, shutdown) = tokio::sync::oneshot::channel::<()>();

    let handle = tokio::spawn(async move {
        harvest(
            provider,
            ReadFrom::Beginning,
            checkpointer,
            conveyor,
            shutdown.map(|_| ()),
        )
        .await
        .unwrap();
    });

    tokio::signal::ctrl_c().await.unwrap();

    trigger.send(()).unwrap();

    handle.await.unwrap();
}
