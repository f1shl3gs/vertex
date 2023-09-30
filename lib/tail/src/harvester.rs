use std::collections::BTreeMap;
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use bytes::Bytes;
use chrono::{DateTime, Utc};
use futures::future::{select, Either};
use futures::{stream, Sink, SinkExt};
use tokio::time::sleep;

use super::checkpoint::{Checkpointer, CheckpointsView, Fingerprint};
use super::watch::Watcher;
use super::ReadFrom;
use crate::provider::Provider;

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

/// `Harvester` as Source
///
/// The `run` of `Harvester` performs the cooperative scheduling of reads over `Harvester`'s
/// configured files. Much care has been taking to make this scheduling `fair`, meaning busy
/// files do not drown out quiet files or vice versa but there's no one perfect approach.
/// Very fast files _will_ be lost if your system aggressively rolls log files. `Harvester` will
/// keep a file handler open but should your system move so quickly that a file disappears
/// before `Harvester` is able to open it the contents will be lost. This should be a race
/// occurrence.
///
/// Specific operation systems support evented interfaces that correct this problem but your
/// intrepid authors know of no generic solution
///
/// `Note`: rotate by `truncating` is not a good solution, so `Truncation is not supporter`.
impl<P> Harvester<P>
where
    P: Provider,
{
    // The first `shutdown` signal here is to stop this harvester from outputting
    // new data; the second `shutdown_checkpointer` is for finishing the background
    // checkpoint writer task, which has to wait for all acknowledgements to be
    // completed.
    pub fn run<C, S1, S2>(
        self,
        mut chans: C,
        mut shutdown: S1,
        shutdown_checkpointer: S2,
        mut checkpointer: Checkpointer,
    ) -> Result<Shutdown, <C as Sink<Vec<Line>>>::Error>
    where
        C: Sink<Vec<Line>> + Unpin,
        <C as Sink<Vec<Line>>>::Error: std::error::Error,
        S1: Future + Unpin + Send + 'static,
        S2: Future + Unpin + Send + 'static,
    {
        checkpointer.read_checkpoints(self.ignore_before);

        // stats
        let mut backoff_cap = 1usize;
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

        let sleep_duration = Duration::from_secs(1);
        let checkpoint_task_handle = self.handle.spawn(checkpoint_writer(
            checkpointer,
            sleep_duration,
            shutdown_checkpointer,
        ));

        let mut next_scan = Instant::now();
        loop {
            // Find files to follow, but not too often
            let now = Instant::now();
            if next_scan <= now {
                // Schedule the next scan time.
                next_scan = now.checked_add(Duration::from_secs(1)).unwrap();

                // Start scan
                for watcher in watchers.values_mut() {
                    watcher.set_findable(false); // assume not findable until found
                }

                for path in self.provider.scan() {
                    if let Ok(fp) = Fingerprint::try_from(&path) {
                        if let Some(watcher) = watchers.get_mut(&fp) {
                            // file fingerprint matches a watched file
                            let was_found_this_cycle = watcher.file_findable();
                            watcher.set_findable(true);
                            if watcher.path == path {
                                trace!(message = "Continue watching file", ?path);
                            } else {
                                // matches a file with a different path
                                if !was_found_this_cycle {
                                    info!(
                                        message = "Watched file has been renamed",
                                        ?path,
                                        old_path = ?watcher.path
                                    );

                                    // ok if this fails: it might be fixed next cycle
                                    watcher.update_path(path).ok();
                                } else {
                                    info!(
                                        message = "More than one file has the same fingerprint",
                                        ?path,
                                        old_path = ?watcher.path
                                    );

                                    let (old, new) = (&watcher.path, &path);
                                    if let (Ok(old_modified_time), Ok(new_modified_time)) = (
                                        std::fs::metadata(old).and_then(|m| m.modified()),
                                        std::fs::metadata(new).and_then(|m| m.modified()),
                                    ) {
                                        if old_modified_time < new_modified_time {
                                            info!(
                                                message = "Switching to watch most recently modified file",
                                                ?new_modified_time,
                                                ?old_modified_time
                                            );

                                            // ok if this fails: it might be fix next cycle
                                            watcher.update_path(path).ok();
                                        }
                                    }
                                }
                            }
                        } else {
                            // untracked file fingerprint
                            self.watch_new_file(path, fp, &mut watchers, &checkpoints, false);
                        }
                    }
                }
            }

            // Collect lines by polling files
            let mut bytes_read = 0usize;
            let mut maxed_out_reading_single_file = false;
            for (&fingerprint, watcher) in &mut watchers {
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

            if !lines.is_empty() {
                let sending = std::mem::take(&mut lines);
                let mut stream = stream::once(futures::future::ok(sending));
                if let Err(err) = self.handle.block_on(chans.send_all(&mut stream)) {
                    error!(
                        message = "Output channel closed",
                        %err
                    );

                    return Err(err);
                }
            }

            // When no lines have been read we kick the backup_cap up by twice,
            // limited by the hard-coded cap. Else, we set the backup_cap to its
            // minimum on the assumption that next time through there will be more
            // lines to read promptly.
            backoff_cap = if bytes_read == 0 {
                std::cmp::min(2048, backoff_cap.saturating_mul(2))
            } else {
                1
            };
            let backoff = backoff_cap.saturating_sub(bytes_read);

            // This works only if run inside tokio context since we are using tokio's timer.
            // Outside of such context, this will panic on the first call. Also since we are using
            // block_on here and in the above code, this should be run in its own thread.
            // `spawn_blocking` fulfills all of these requirements.
            let sleep = async move {
                if backoff > 0 {
                    sleep(Duration::from_millis(backoff as u64)).await;
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
                debug!(message = "Staring watch file", path = ?path);
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

async fn checkpoint_writer(
    checkpointer: Checkpointer,
    sleep_duration: Duration,
    mut shutdown: impl Future + Unpin,
) -> Arc<Checkpointer> {
    let checkpointer = Arc::new(checkpointer);

    loop {
        let sleep = sleep(sleep_duration);
        tokio::select! {
            _ = &mut shutdown => break,
            _ = sleep => {},
        }

        let checkpointer = Arc::clone(&checkpointer);
        tokio::task::spawn_blocking(move || {
            if let Err(err) = checkpointer.write_checkpoints() {
                error!(
                    message = "Failed writing checkpoints",
                    %err
                );
            }
        })
        .await
        .ok();
    }

    checkpointer
}
