use std::fs::{File, read_dir};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use futures::{FutureExt, StreamExt};
use tail::decode::NewlineDecoder;
use tail::{Checkpointer, Conveyor, FileReader, ReadFrom, Shutdown, harvest};
use temp_dir::TempDir;
use tokio::time::Interval;
use tokio_util::codec::FramedRead;

struct Provider {
    root: PathBuf,
    active: Option<PathBuf>,

    interval: Interval,
}

impl tail::Provider for Provider {
    type Metadata = ();

    async fn scan(&mut self) -> std::io::Result<Vec<(PathBuf, Self::Metadata)>> {
        self.interval.tick().await;

        match &self.active {
            Some(file) => Ok(vec![(self.root.join(file), ())]),
            None => {
                let mut files = vec![];

                for entry in read_dir(&self.root)?.flatten() {
                    let path = entry.path();
                    let Some(ext) = path.extension() else {
                        continue;
                    };

                    if ext == "log" {
                        files.push((entry.path(), ()))
                    }
                }

                Ok(files)
            }
        }
    }
}

#[derive(Clone, Default)]
struct OrderedOutput {
    seq: Arc<AtomicU64>,
}

impl Conveyor for OrderedOutput {
    type Metadata = ();

    fn run(
        &self,
        reader: FileReader,
        _meta: Self::Metadata,
        _offset: Arc<AtomicU64>,
        shutdown: Shutdown,
    ) -> impl Future<Output = Result<(), ()>> + Send + 'static {
        let mut reader = FramedRead::new(reader, NewlineDecoder::new(4 * 1024))
            .take_until(shutdown);
        let seq = Arc::clone(&self.seq);

        Box::pin(async move {
            while let Some(Ok((data, _size))) = reader.next().await {
                let want = seq.load(Ordering::Acquire);
                let got = String::from_utf8_lossy(&data).parse::<u64>().unwrap();

                println!("consume {got}");

                assert_eq!(want, got);

                seq.fetch_add(1, Ordering::Release);
            }

            Ok(())
        })
    }
}

async fn run_with<P, F>(active: Option<PathBuf>, produce: P)
where
    P: FnOnce(PathBuf) -> F + Send + 'static,
    F: Future<Output = ()> + Send + 'static,
{
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .init();

    let root = TempDir::new().unwrap();
    let checkpointer = Checkpointer::load(root.path().to_path_buf()).unwrap();

    let provider = Provider {
        root: root.path().to_path_buf(),
        active,
        interval: tokio::time::interval(Duration::from_millis(100)),
    };

    let (trigger, shutdown) = tokio::sync::oneshot::channel();

    tokio::spawn(async move {
        produce(root.path().to_path_buf()).await;
        trigger.send(()).unwrap();
    });

    harvest(
        provider,
        ReadFrom::Beginning,
        checkpointer,
        OrderedOutput::default(),
        shutdown.map(|_| ()),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn create_new() {
    let rotate = 5;
    let lines = 10;

    run_with(None, async move |root: PathBuf| {
        for i in 0..rotate {
            let mut file = File::create_new(root.join(format!("{i}.log"))).unwrap();

            for j in 0..lines {
                let seq = i * lines + j;

                file.write_all(format!("{seq}\n").as_bytes()).unwrap();

                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }
    })
    .await;
}

#[tokio::test]
async fn active_only() {
    let rotate = 5;
    let lines = 10;

    run_with(Some("test.log".into()), async move |root: PathBuf| {
        for i in 0..rotate {
            let mut file = File::create_new(root.join("test.log")).unwrap();

            for j in 0..lines {
                let seq = i * lines + j;

                file.write_all(format!("{seq}\n").as_bytes()).unwrap();

                tokio::time::sleep(Duration::from_millis(100)).await;
            }

            std::fs::rename(root.join("test.log"), root.join(format!("test.log.{i}"))).unwrap();
        }
    })
    .await
}

#[tokio::test]
async fn all() {
    let rotate = 5;
    let lines = 10;

    run_with(None, async move |root: PathBuf| {
        for i in 0..rotate {
            let mut file = File::create_new(root.join("test.log")).unwrap();

            for j in 0..lines {
                let seq = i * lines + j;

                file.write_all(format!("{seq}\n").as_bytes()).unwrap();

                tokio::time::sleep(Duration::from_millis(100)).await;
            }

            std::fs::rename(root.join("test.log"), root.join(format!("test.{i}.log"))).unwrap();
        }
    })
    .await
}
