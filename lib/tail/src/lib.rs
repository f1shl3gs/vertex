mod buffer;
mod checkpoint;
mod scan;
mod watch;

// re-export
pub use checkpoint::{Fingerprint, Position};

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::sleep;
use std::time::{Duration, Instant};

use bytes::Bytes;
use futures::{Future, FutureExt, Sink, SinkExt, StreamExt};
use tokio_stream::wrappers::IntervalStream;
use tracing::{error, warn};

use crate::checkpoint::{Checkpointer, CheckpointsView};
use crate::scan::Provider;
use crate::watch::Watcher;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ReadFrom {
    Beginning,
    End,
    Checkpoint(Position),
}

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
    remove_after: Option<Duration>,
    read_from: ReadFrom,
    ignore_checkpoints: bool,
    oldest_first: bool,
    glob_cooldown: Duration,
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
        let handle = tokio::runtime::Handle::current();
        let mut fps: BTreeMap<Fingerprint, Watcher> = BTreeMap::new();

        let mut existing_files = Vec::new();
        for path in self.provider.scan() {
            if let Ok(fp) = Fingerprint::try_from(&path) {
                existing_files.push((path, fp));
            }
        }

        let checkpoints = checkpointer.view();
        for (path, fp) in existing_files {
            self.watch_new_file(path.clone(), fp, &mut fps, &checkpoints, true);
        }

        // Spawn the checkpoint writer task
        //
        // We have to do a lot of cloning here to convince the compiler that we aren't going to
        // get away with anything, but none of it should have any perf impact.
        let mut shutdown = shutdown.shared();
        let mut shutdown2 = shutdown.clone();
        let checkpointer = Arc::new(checkpointer);
        let sleep_duration = std::time::Duration::from_secs(1);
        let interval = tokio::time::interval(sleep_duration);
        let checkpoint_task_handle = tokio::spawn(async move {
            let mut ticker = IntervalStream::new(interval).take_until(shutdown2);

            while ticker.next().await.is_some() {
                // todo: Observability
                checkpointer.persist();
            }

            let checkpointer = Arc::clone(&checkpointer);
            checkpointer.persist();

            checkpointer
        });

        // We want to avoid burning up user's CPUs. To do this we sleep after reading lines
        // out of files. But! We want to be responsive as well. We keep track of a 'backoff_cap'
        // to decide how long we'll wait in any given loop. This cap grows each time we fail
        // to read lines in an exponential fashion to some hard-coded cap. To reduce time
        // using glob, we do not re-scan for major file changes(new files, moves, deletes),
        // or write new checkpoints, on every iteration.
        let mut next_glob_time = Instant::now();
        loop {
            // Glob find files to follow, but not too often.
            let now_time = Instant::now();
            if next_glob_time <= now_time {
                // Schedule the next glob time.
                next_glob_time = now_time.checked_add(self.glob_cooldown).unwrap();
            }
        }
    }

    fn watch_new_file(
        &self,
        path: PathBuf,
        fp: Fingerprint,
        fps: &mut BTreeMap<Fingerprint, Watcher>,
        checkpoints: &CheckpointsView,
        startup: bool,
    ) {
        // Determine the initial _requested_ starting point in the file. This can be overridden
        // once the file is actually opened and we determine it is compressed, older than we're
        // configured to read, etc
        let fallback = if startup {
            self.read_from
        } else {
            // Always read new files that show up while we're running from the beginning. There's
            // not a good way to determine if they moved or just created and written very quickly,
            // so just make sure we're not missing any data.
            ReadFrom::Beginning
        };

        // Always prefer the stored checkpoints unless the user has opted out. Previously, the
        // checkpoint was only loaded for new files when server was started up.
        let read_from = if !self.ignore_checkpoints {
            checkpoints
                .get(fp)
                .map(ReadFrom::Checkpoint)
                .unwrap_or(fallback)
        } else {
            fallback
        };

        match Watcher::new(
            path.clone(),
            read_from,
            self.ignore_before,
            self.max_line_bytes,
            self.line_delimiter.clone(),
        ) {
            Ok(mut watcher) => {
                if let ReadFrom::Checkpoint(pos) = read_from {} else {}

                watcher.set_findable(true);
                fps.insert(fp, watcher);
            }
            Err(err) => {
                warn!(
                    message = "Create new file watcher failed",
                    ?err
                )
            }
        }
    }
}

/// A sentinel type to signal that Server was gracefully shutdown
///
/// The purpose of this type is to clarify the semantics of the result values
/// returned from the [`Server::run`] for both the users of the file server,
/// and the implementors
#[derive(Debug)]
pub struct Shutdown;
