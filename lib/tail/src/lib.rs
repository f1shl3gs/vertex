mod watch;
mod scan;
mod checkpoint;
mod buffer;

// re-export
pub use checkpoint::{Position, Fingerprint};

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use futures::{Future, Sink, StreamExt};
use bytes::Bytes;
use tokio_stream::wrappers::IntervalStream;

use crate::checkpoint::{Checkpointer, CheckpointsView};
use crate::scan::Provider;
use crate::watch::Watcher;

#[derive(Debug)]
pub struct Line {
    pub text: Bytes,
    pub filename: String,
    pub fingerprint: Fingerprint,
    pub offset: u64,
}

pub struct Server {
    provider: scan::Glob,
    max_read_bytes: usize,
    remove_after: Option<Duration>
}

impl Server {
    pub async fn run<C, S>(
        self,
        mut chan: C,
        shutdown: S,
        mut checkpointer: Checkpointer,
    ) -> Result<Shutdown, <C as Sink<Vec<Line>>>::Error>
        where
            C: Sink<Vec<Line>> + Unpin,
            <C as Sink<Vec<Line>>>::Error: std::error::Error,
            S: Future + Unpin + Send + 'static,
            <S as Future>::Output: Clone + Send + Sync,
    {
        // some global stats
        let mut fps: BTreeMap<Fingerprint, Watcher> = BTreeMap::new();

        let mut existing_files = Vec::new();
        for path in self.provider.scan() {
            if let Ok(fp) = Fingerprint::try_from(path) {
                existing_files.push((path, fp));
            }
        }

        let checkpoints = checkpointer.view();
        for (path, fp) in existing_files {
            self.watch_new_file(path, fp, &mut fps, &checkpoints, true);
        }

        // Spawn the checkpoint writer task
        //
        // We have to do a lot of cloning here to convince the compiler that we aren't going to
        // get away with anything, but none of it should have any perf impact.
        let mut shutdown = shutdown.clone();
        let mut shutdown2 = shutdown.clone();
        let checkpointer = Arc::new(checkpointer);
        let sleep_duration = std::time::Duration::from_secs(1);
        let interval = tokio::time::interval(sleep_duration);
        tokio::spawn(async move {
            let mut ticker = IntervalStream::new(interval).take_until(shutdown2);

            while ticker.next().await.is_some() {
                // todo: Observability
                checkpointer.flush()
            }
        });

        let mut lines = Vec::with_capacity(1024);
        let mut ticker = IntervalStream::new(interval.clone()).take_until(shutdown);
        while ticker.next().await.is_some() {
            let mut global_bytes_read: usize = 0;

            for (&fp, watcher) in &mut fps {
                if !watcher.should_read() {
                    continue;
                }

                let mut bytes_read = 0usize;
                while let Ok(Some(line)) = watcher.read_line() {
                    let size = line.len();
                    bytes_read += size;
                    lines.push(Line {
                        text: line,
                        filename: "todo".to_owned(), // alloc here is dummy
                        fingerprint: fp,
                        offset: watcher.file_position(),
                    });

                    if bytes_read > self.max_read_bytes {
                        break;
                    }
                }

                if bytes_read > 0 {
                    global_bytes_read = global_bytes_read.saturating_add(bytes_read);
                } else {
                    // should the file be removed?
                    if let Some(grace_period) = self.remove_after {}
                }
            }
        }

        todo!()
    }

    fn watch_new_file(
        &self,
        path: PathBuf,
        fp: Fingerprint,
        fps: &mut BTreeMap<Fingerprint, Watcher>,
        checkpoints: &CheckpointsView,
        startup: bool)
    {
        todo!()
    }
}

/// A sentinel type to signal that Server was gracefully shutdown
///
/// The purpose of this type is to clarify the semantics of the result values
/// returned from the [`Server::run`] for both the users of the file server,
/// and the implementors
#[derive(Debug)]
pub struct Shutdown;


#[cfg(test)]
mod tests {
    use std::path::Path;
    use tokio_stream::StreamExt;
    use tokio_stream::wrappers::IntervalStream;
    use crate::scan::{Glob, Provider};
    use super::*;
    use futures::Sink;


    #[tokio::test]
    async fn run() {
        let path = Path::new("./");
        let interval = std::time::Duration::from_secs(5);
        let checkpointer = checkpoint::Checkpointer::new(path);
        let provider = scan::Glob::new(&["*.log".into()], &["exclude.log".into()]).unwrap();

        tokio::spawn(async move {
            let interval = tokio::time::interval(interval);
            let mut ticker = IntervalStream::new(interval);

            while ticker.next().await.is_some() {
                let paths = provider.scan();
                println!("{:?}", paths);
            }
        });

        tokio::time::sleep(tokio::time::Duration::from_secs(6)).await
    }
}