use crate::{Checkpointer, Harvester, Line};
use bstr::ByteSlice;
use bytes::Bytes;
use futures::channel::oneshot;
use futures::{ready, FutureExt, StreamExt};
use std::future::Future;
use std::io::Write;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};
use tempfile::tempdir;

pub struct ShutdownHandle(oneshot::Receiver<()>);

impl Future for ShutdownHandle {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let _ = ready!(self.0.poll_unpin(cx));
        Poll::Ready(())
    }
}

fn parse_sequence(line: Bytes) -> u64 {
    let s = line.to_str().unwrap();
    let parts = s.split_ascii_whitespace().collect::<Vec<_>>();
    assert_eq!(parts.len(), 2);
    parts[0].parse().unwrap()
}

#[tracing_test::traced_test]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_move_and_create() {
    let log_count = 10;
    let log_size = 100;
    let mut writes = Arc::new(AtomicU64::new(0));
    let mut reads = Arc::new(AtomicU64::new(0));
    let tempdir = tempdir().unwrap();
    let dir = tempdir.path();

    let include = vec![dir.join("*.log*")];

    let provider = crate::provider::Glob::new(&include, &[]).unwrap();

    let harvester = Harvester {
        provider,
        read_from: Default::default(),
        max_read_bytes: 2048,
        handle: tokio::runtime::Handle::current(),
        ignore_before: None,
        max_line_bytes: 4096,
        line_delimiter: Bytes::from("\n"),
    };

    let (shutdown_trigger, r) = oneshot::channel();
    let shutdown = ShutdownHandle(r);

    let (tx, mut rx) = futures::channel::mpsc::channel::<Vec<Line>>(2);
    let checkpointer = Checkpointer::new(&dir);

    // start writer
    let log_path = dir.join("test.log");
    let cw = Arc::clone(&writes);
    tokio::spawn(async move {
        for i in 0..log_count {
            let mut f = std::fs::File::create(&log_path).unwrap();
            for j in 0..log_size {
                write!(f, "{} abcedefghijklmnopqrstuvwxyz0123456789\n", i * 100 + j)
                    .expect("write log success");
                cw.fetch_add(1, Ordering::Relaxed);
            }

            // rotate
            let to_path = format!("{}.{}", log_path.to_str().unwrap(), i);
            std::fs::rename(&log_path, to_path).unwrap();

            // waiting for glob scan the new file
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }

        shutdown_trigger.send(()).unwrap();
    });

    // start reader
    let cr = Arc::clone(&reads);
    tokio::spawn(async move {
        let mut want = 0;

        while let Some(lines) = rx.next().await {
            for line in lines {
                let seq = parse_sequence(line.text);
                assert_eq!(seq, want);
                want = seq + 1;
                cr.fetch_add(1, Ordering::Relaxed);
            }
        }
    });

    tokio::task::spawn_blocking(move || {
        let result = harvester.run(tx, shutdown, checkpointer);
        result.unwrap();
    })
    .await
    .unwrap();

    let writes = writes.load(Ordering::Relaxed);
    let reads = reads.load(Ordering::Relaxed);
    assert_eq!(writes, log_size * log_count);
    assert_eq!(writes, reads);
}
