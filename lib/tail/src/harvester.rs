use bytes::Bytes;
use chrono::{DateTime, Utc};
use futures::future::{select, Either, FutureExt};
use futures::{stream, Sink, SinkExt};
use std::collections::BTreeMap;
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, error, warn};

use super::checkpoint::{Checkpointer, CheckpointsView, Fingerprint};
use super::watch::Watcher;
use crate::provider::Provider;
use crate::ReadFrom;

/// A sentinel type to signal that file server was gracefully shut down.
///
/// The purpose of this type is to clarify the semantics of the result values
/// returned from the [`Harvester::run`] for both the users of the file server,
/// and the implementors.
#[derive(Debug)]
pub struct Shutdown;

#[derive(Debug)]
pub struct Line {
    pub text: Bytes,
    pub filename: String,
    pub fingerprint: Fingerprint,
    pub offset: u64,
}

pub struct Harvester<P>
where
    P: Provider,
{
    pub provider: P,
    pub read_from: ReadFrom,
    pub max_read_bytes: usize,
    pub handle: tokio::runtime::Handle,

    pub ignore_before: Option<DateTime<Utc>>,
    pub max_line_bytes: usize,
    pub line_delimiter: Bytes,
}

impl<P> Harvester<P>
where
    P: Provider,
{
    pub fn run<C, S>(
        self,
        mut chans: C,
        shutdown: S,
        mut checkpointer: Checkpointer,
    ) -> Result<Shutdown, <C as Sink<Vec<Line>>>::Error>
    where
        C: Sink<Vec<Line>> + Unpin,
        <C as Sink<Vec<Line>>>::Error: std::error::Error,
        S: Future + Unpin + Send + 'static,
        <S as Future>::Output: Clone + Send + Sync,
    {
        checkpointer.read_checkpoints(self.ignore_before);

        // stats
        let mut existing = Vec::new();
        let mut watchers: BTreeMap<Fingerprint, Watcher> = Default::default();
        let mut lines = vec![];

        // first scan
        for path in self.provider.scan() {
            match Fingerprint::try_from(&path) {
                Ok(fp) => existing.push((path, fp)),
                Err(err) => {
                    warn!(message = "Convert fingerprint from file failed", ?err);

                    continue;
                }
            }
        }

        let checkpoints = checkpointer.view();
        for (path, fp) in existing {
            self.watch_new_file(path, fp, &mut watchers, &checkpoints, true);
        }

        // Spawn the checkpoint writer task
        //
        // We have to do a lot of cloning here to convince the compiler that we aren't
        // going to get away with anything, but none of it should have any perf impact.
        let mut shutdown = shutdown.shared();
        let mut shutdown2 = shutdown.clone();
        let checkpointer = Arc::new(checkpointer);
        let sleep_duration = std::time::Duration::from_secs(3);
        let checkpoint_task_handle = self.handle.spawn(async move {
            loop {
                let sleep = tokio::time::sleep(sleep_duration);
                tokio::select! {
                    _ = &mut shutdown2 => return checkpointer,
                    _ = sleep => {}
                }

                let checkpointer = Arc::clone(&checkpointer);
                tokio::task::spawn_blocking(move || {
                    let start = Instant::now();

                    match checkpointer.write_checkpoints() {
                        Ok(count) => debug!(
                            message = "Files checkpointed",
                            count = %count,
                            duration_ms = start.elapsed().as_millis() as u64
                        ),
                        Err(err) => error!(
                            message = "Failed writing checkpoints",
                            %err,
                            duration_ms = start.elapsed().as_millis() as u64
                        ),
                    }
                })
                .await
                .ok();
            }
        });

        //

        loop {
            // Collect lines by polling files
            let mut bytes_read = 0usize;
            let mut maxed_out_reading_single_file = false;
            for (&fingerprint, watcher) in &mut watchers {
                if !watcher.should_read() {
                    continue;
                }

                while let Ok(Some(line)) = watcher.read_line() {
                    bytes_read += line.len();
                    lines.push(Line {
                        text: line,
                        fingerprint,
                        filename: watcher.path.to_str().expect("not a valid path").to_owned(),
                        offset: watcher.file_position(),
                    });

                    if bytes_read > self.max_read_bytes {
                        maxed_out_reading_single_file = true;
                        break;
                    }
                }

                if maxed_out_reading_single_file {
                    break;
                }
            }

            // Watcher is dead when the underlying file has disappeared.
            // If the Watcher is dead we don't retain it; it will be deallocated.
            watchers.retain(|fp, watcher| {
                if watcher.dead() {
                    checkpoints.set_dead(*fp);
                    false
                } else {
                    true
                }
            });

            let sending = std::mem::take(&mut lines);
            let mut stream = stream::once(futures::future::ok(sending));
            if let Err(err) = self.handle.block_on(chans.send_all(&mut stream)) {
                error!(
                    message = "Output channel closed",
                    %err
                );

                return Err(err);
            }

            // TODO: implement backoff
            let backoff = 500; // 500ms

            let sleep = async move {
                if backoff > 0 {
                    tokio::time::sleep(Duration::from_millis(backoff as u64)).await;
                }
            };
            futures::pin_mut!(sleep);
            match self.handle.block_on(select(shutdown, sleep)) {
                Either::Left((_, _)) => {
                    let checkpointer = self
                        .handle
                        .block_on(checkpoint_task_handle)
                        .expect("checkpoint task has panicked");

                    if let Err(err) = checkpointer.write_checkpoints() {
                        error!(message = "Error writing checkpoints before shutdown", ?err);
                    }

                    return Ok(Shutdown);
                }
                Either::Right((_, future)) => shutdown = future,
            }
        }
    }

    fn watch_new_file(
        &self,
        path: PathBuf,
        fp: Fingerprint,
        watchers: &mut BTreeMap<Fingerprint, Watcher>,
        checkpoints: &CheckpointsView,
        startup: bool,
    ) {
        // Determine the initial _requested_ starting point in the file. This can be overridden
        // once the file is actually opened and we determine it is compressed, older thant we're
        // configured to read, etc.
        let fallback = if startup {
            self.read_from
        } else {
            // Always read new files that show up while we're running from the beginning. There's
            // not a good way to determine if they were moved or just created and written very
            // quickly, so just make sure we're not missing any data.
            ReadFrom::Beginning
        };

        let read_from = checkpoints
            .get(fp)
            .map(ReadFrom::Checkpoint)
            .unwrap_or(fallback);

        match Watcher::new(
            path.clone(),
            read_from,
            self.ignore_before,
            self.max_line_bytes,
            self.line_delimiter.clone(),
        ) {
            Ok(mut watcher) => {
                watcher.set_findable(true);
                watchers.insert(fp, watcher);
            }
            Err(err) => {
                error!(
                    message = "Failed to watch file",
                    file = %path.display(),
                    %err
                );
            }
        }
    }
}
