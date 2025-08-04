use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::time::Duration;

use futures::{FutureExt, StreamExt};
use tail::decode::NewlineDecoder;
use tail::{Checkpointer, Conveyor, FileReader, Provider, ReadFrom, Shutdown, harvest};
use temp_dir::TempDir;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::Interval;
use tokio_util::codec::FramedRead;

struct ListProvider {
    root: PathBuf,
    interval: Interval,
}

impl ListProvider {
    fn new(root: PathBuf, interval: Duration) -> Self {
        ListProvider {
            root,
            interval: tokio::time::interval(interval),
        }
    }
}

impl Provider for ListProvider {
    type Metadata = ();

    async fn scan(&mut self) -> std::io::Result<Vec<(PathBuf, Self::Metadata)>> {
        self.interval.tick().await;

        let mut paths = vec![];

        for entry in std::fs::read_dir(&self.root)?.flatten() {
            let path = entry.path();

            let Some(ext) = path.extension() else {
                continue;
            };

            if ext == "log" {
                paths.push((entry.path(), ()));
            }
        }

        Ok(paths)
    }
}

#[derive(Clone)]
struct StdoutOutput;

impl Conveyor for StdoutOutput {
    type Metadata = ();

    fn run(
        &self,
        reader: FileReader,
        _meta: Self::Metadata,
        _offset: Arc<AtomicU64>,
        shutdown: Shutdown,
    ) -> impl Future<Output = Result<(), ()>> + Send + 'static {
        let mut reader =
            FramedRead::new(reader, NewlineDecoder::new(4 * 1024)).take_until(shutdown);

        Box::pin(async move {
            while let Some(Ok((data, _size))) = reader.next().await {
                println!("output: {}", String::from_utf8_lossy(&data));
            }

            Ok(())
        })
    }
}

#[derive(Clone)]
struct TrackedOutput {
    want: usize,
    sender: UnboundedSender<()>,
}

impl Conveyor for TrackedOutput {
    type Metadata = ();

    fn run(
        &self,
        reader: FileReader,
        _meta: Self::Metadata,
        _offset: Arc<AtomicU64>,
        shutdown: Shutdown,
    ) -> impl Future<Output = Result<(), ()>> + Send + 'static {
        let mut reader =
            FramedRead::new(reader, NewlineDecoder::new(4 * 1024)).take_until(shutdown);
        let sender = self.sender.clone();
        let want = self.want;

        Box::pin(async move {
            let mut consumed = 0;

            while let Some(Ok((_data, size))) = reader.next().await {
                consumed += size;

                if consumed >= want {
                    sender.send(()).unwrap();
                }
            }

            Ok(())
        })
    }
}

#[tokio::test]
async fn rename_then_create() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .init();

    let root = TempDir::new().unwrap();
    std::fs::create_dir_all(&root).unwrap();

    let provider = ListProvider::new(root.path().to_path_buf(), Duration::from_millis(100));

    let checkpointer = Checkpointer::load(root.child("checkpoints")).unwrap();

    let (trigger, shutdown) = tokio::sync::oneshot::channel::<()>();

    let handle = tokio::spawn(async move {
        harvest(
            provider,
            ReadFrom::Beginning,
            checkpointer,
            StdoutOutput,
            shutdown.map(|_| ()),
        )
        .await
        .unwrap();
    });

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(root.child("1.log"))
        .unwrap();

    for i in 0..10 {
        let line = format!("line {}\n", i + 1);
        file.write_all(line.as_bytes()).unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    std::fs::rename(root.child("1.log"), root.child("2.log")).unwrap();
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(root.child("1.log"))
        .unwrap();

    for i in 10..20 {
        let line = format!("line {}\n", i + 1);
        file.write_all(line.as_bytes()).unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    trigger.send(()).unwrap();

    handle.await.unwrap();
}

#[tokio::test]
async fn cold_file() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .init();

    let root = TempDir::new().unwrap();
    std::fs::create_dir_all(root.path()).unwrap();

    let lines = 10000;
    let mut written = 0;
    // prepare the cold file
    let mut file = std::fs::File::create(root.child("1.log")).unwrap();
    for i in 0..lines {
        let line = format!(
            "{i:04} S2oV1HbFPd2zDPfYkuhaICbfe1hm5lke1C6DmCeUSJgLZl0fze1gRuRfOrJauDEABCnX8HRRbp9rDtSwOuGhvrzvcXEniDWXheDT\n"
        );
        file.write_all(line.as_bytes()).unwrap();

        written += line.len();
    }
    file.flush().unwrap();

    let provider = ListProvider::new(root.path().to_path_buf(), Duration::from_millis(100));
    let checkpointer = Checkpointer::load(root.child("checkpoints")).unwrap();

    let (sender, mut rx) = tokio::sync::mpsc::unbounded_channel::<()>();
    let tracked_output = TrackedOutput {
        want: written,
        sender,
    };

    harvest(
        provider,
        ReadFrom::Beginning,
        checkpointer,
        tracked_output,
        Box::pin(async move {
            rx.recv().await.unwrap();
        }),
    )
    .await
    .unwrap();
}
