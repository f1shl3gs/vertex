use std::collections::{BTreeMap, HashSet};
use chrono::{DateTime, Utc};
use bytes::{Bytes};
use std::{
    cmp,
    fs,
    time::{self, Duration},
    sync::Arc,
};
use std::fs::remove_file;
use std::path::PathBuf;
use futures::{
    future::{
        select, Either, FutureExt,
    },
    stream,
    Future,
    Sink,
    SinkExt,
};
use tracing::{debug, Instrument, info, error};
use indexmap::map::IndexMap;
use crate::checkpointer::{Checkpointer, CheckpointsView};
use crate::events::InternalEvents;
use crate::fingerprinter::{Fingerprint, Fingerprinter};
use crate::provider::Provider;
use crate::ReadFrom;
use crate::watcher::Watcher;

/// `Server` is a Source which coopeatively schedules reads over files,
/// converting the lines of said files into `LogLine` structures. As
/// `Server` is intended to be useful across multiple operating systems with
/// POSIX filesystem semantics `Server` must poll for changes. That is, no
/// event notification is used by `Server`
///
/// `Server` is configured on a path to watch. The files do _not_ need to
/// exist at startup. `Server` will discover new files which match its
/// path in at most 60 seconds.
pub struct Server<P: Provider, E: InternalEvents> {
    pub provider: P,
    pub max_read_bytes: usize,
    pub ignore_checkpoints: bool,
    pub read_from: ReadFrom,
    pub ignore_before: Option<DateTime<Utc>>,
    pub max_line_bytes: usize,
    pub line_delimiter: Bytes,
    pub glob_minimum_cooldown: Duration,
    pub fingerprinter: Fingerprinter,
    pub oldest_first: bool,
    pub remove_after: Option<Duration>,
    pub emitter: E,
    pub handle: tokio::runtime::Handle,
}

/// `Server` as Source
///
/// The `run` of `Server` performs the cooperative scheduling of reads over
/// `Server`'s configured files. Much care has been taking to make this
/// scheduling 'fair', meaning busy files do not drown out quiet files or
/// vice versa but there's no one perfect approach. Very fast files _will_
/// be lost if your system aggressively rolls log files. `Server` will keep
/// a file handler open but should your system move so quickly that a file
/// disappears before `Server` is able to open it the contents will be lost.
/// This should be a rare occurrence.
///
/// Specific operating systems support evented interfaces that correct this
/// problem but your intrepid authors know of no generic solution
impl<P, E> Server<P, E>
    where
        P: Provider,
        E: InternalEvents,
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
        let mut fingerprint_buffer = Vec::new();
        // let mut fps: IndexMap<Fingerprint, Watcher> = Default::default();
        let mut fps: BTreeMap<Fingerprint, Watcher> = BTreeMap::new();
        let mut backoff_cap = 1usize;
        let mut lines = Vec::new();

        checkpointer.read_checkpoints(self.ignore_before);

        let mut known_small_files = HashSet::new();
        let mut existing_files = Vec::new();
        for path in self.provider.paths().into_iter() {
            if let Some(fp) = self.fingerprinter.get_fingerprint_or_log_error(
                &path,
                &mut fingerprint_buffer,
                &mut known_small_files,
                &self.emitter,
            ) {
                existing_files.push((path, fp))
            }
        }

        existing_files.sort_by_key(|(path, _fp)| {
            fs::metadata(&path)
                .and_then(|m| m.created())
                .map(DateTime::<Utc>::from)
                .unwrap_or_else(|_| Utc::now())
        });

        let checkpoints = checkpointer.view();
        for (path, fp) in existing_files {
            checkpointer.maybe_upgrade(
                &path,
                fp,
                &self.fingerprinter,
                &mut fingerprint_buffer,
            );

            self.watch_new_file(path, fp, &mut fps, &checkpoints, true);
        }
        self.emitter.emit_files_open(fps.len());

        let mut stats = TimingStats::default();

        // Spawn the checkpoint writer task
        //
        // We have to do a lot of cloning here to convince the compiler that we
        // aren't going to get away with anything, but none of it should have
        // any perf impact.
        let mut shutdown = shutdown.shared();
        let mut shutdown2 = shutdown.clone();
        let emitter = self.emitter.clone();
        let checkpointer = Arc::new(checkpointer);
        let sleep_duration = self.glob_minimum_cooldown;
        let checkpoint_task_handle = self.handle.spawn(async move {
            loop {
                let sleep = tokio::time::sleep(sleep_duration);
                tokio::select! {
                    _ = &mut shutdown2 => return checkpointer,
                    _ = sleep => {},
                }

                let emitter = emitter.clone();
                let checkpointer = Arc::clone(&checkpointer);
                tokio::task::spawn_blocking(move || {
                    let start = time::Instant::now();
                    match checkpointer.write_checkpoints() {
                        Ok(count) => emitter.emit_file_checkpointed(count, start.elapsed()),
                        Err(err) => emitter.emit_file_checkpoint_write_failed(err),
                    }
                }).await.ok();
            }
        });

        // Alright, how does this work?
        //
        // We want to avoid burning up users' CPUs. To do this we sleep after
        // reading lines out of files. But we want to be responsive as well.
        // We keep track of a 'backoff_cap' to decide how long we'll wait in
        // any given loop. This cap grows each time we fail to read lines in
        // an exponential fashion to some hard-coded cap. To reduce time using
        // glob, we do not re-scan for major file changes (new files, moves, deletes),
        // or write new checkpoints, on every iteration.
        let mut next_glob_time = time::Instant::now();
        loop {
            // Glob find files to follow, but not too often
            let now = time::Instant::now();
            if next_glob_time <= now {
                // Schedule the next glob time.
                next_glob_time = now.checked_add(self.glob_minimum_cooldown).unwrap();
                if stats.started_at.elapsed() > Duration::from_secs(1) {
                    stats.report();
                }

                if stats.started_at.elapsed() > Duration::from_secs(10) {
                    stats = TimingStats::default();
                }

                // Search(glob) for files to detect major file changes
                let start = time::Instant::now();
                for (fp, watcher) in &mut fps {
                    watcher.set_file_findable(false); // assume not findable until found
                }

                let paths = self.provider.paths();
                for path in paths.into_iter() {
                    if let Some(fp) = self.fingerprinter.get_fingerprint_or_log_error(
                        &path,
                        &mut fingerprint_buffer,
                        &mut known_small_files,
                        &self.emitter,
                    ) {
                        if let Some(watcher) = fps.get_mut(&fp) {
                            // file fingerprint matches a watched file
                            let was_found_this_cycle = watcher.file_findable();
                            watcher.set_file_findable(true);
                            if watcher.path == path {
                                // trace!?
                            } else {
                                // matches a file with a different path
                                if !was_found_this_cycle {
                                    info!(
                                        message = "More than one file has the same fingerprint",
                                        path = ?path,
                                        old_path = ?watcher.path
                                    );

                                    let (old, new) = (&watcher.path, &path);
                                    if let (Ok(old_modified_time), Ok(new_modified_time)) = (
                                        fs::metadata(&old).and_then(|m| m.modified()),
                                        fs::metadata(&new).and_then(|m| m.modified())
                                    ) {
                                        if old_modified_time < new_modified_time {
                                            info!(
                                                message = "Switching to watch most recently modified file",
                                                new = ?new_modified_time,
                                                old = ?old_modified_time
                                            );

                                            // ok if this fails: might fix next cycle
                                            watcher.update_path(path).ok();
                                        }
                                    }
                                }
                            }
                        } else {
                            // untracked file fingerprint
                            self.watch_new_file(path, fp, &mut fps, &checkpoints, false);
                            self.emitter.emit_files_open(fps.len());
                        }
                    }
                }
                stats.record("discovery", start.elapsed());
            }

            let mut global_bytes_read: usize = 0;
            let mut maxed_out_reading_single_file = false;
            for (&fp, watcher) in &mut fps {
                if !watcher.should_read() {
                    continue;
                }

                let start = time::Instant::now();
                let mut bytes_read = 0usize;
                while let Ok(Some(line)) = watcher.read_line() {
                    let sz = line.len();
                    stats.record_bytes(sz);
                    bytes_read += sz;
                    lines.push(Line {
                        text: line,
                        filename: watcher.path.to_str().expect("not a valid path").to_owned(),
                        fingerprint: fp,
                        offset: watcher.get_file_position(),
                    });

                    if bytes_read > self.max_read_bytes {
                        maxed_out_reading_single_file = true;
                        break;
                    }
                }
                stats.record("reading", start.elapsed());

                if bytes_read > 0 {
                    global_bytes_read = global_bytes_read.saturating_add(bytes_read);
                } else {
                    // should the file be removed
                    if let Some(grace_period) = self.remove_after {
                        if watcher.last_read_success().elapsed() >= grace_period {
                            // Try to remove
                            match remove_file(&watcher.path) {
                                Ok(()) => {
                                    self.emitter.emit_file_deleted(&watcher.path);
                                    watcher.set_dead();
                                }

                                Err(err) => {
                                    // We will try again after some time
                                    self.emitter.emit_file_delete_failed(&watcher.path, err);
                                }
                            }
                        }
                    }
                }

                // Do not move on to newer files if we are behind on an older file
                if self.oldest_first && maxed_out_reading_single_file {
                    break;
                }
            }

            // A Watcher is dead when the underlying file has disappeared.
            // If the Watcher is daed we don't retain it; it will be deallocated.
            fps.retain(|fp, watcher| {
                if watcher.dead() {
                    self.emitter.emit_file_unwatched(&watcher.path);
                    checkpoints.set_dead(*fp);
                    false
                } else {
                    true
                }
            });
            self.emitter.emit_files_open(fps.len());

            let start = time::Instant::now();
            let to_send = std::mem::take(&mut lines);
            let mut stream = stream::once(futures::future::ok(to_send));
            let result = self.handle.block_on(
                chans.send_all(&mut stream)
            );

            match result {
                Ok(()) => {}
                Err(err) => {
                    error!(
                        message = "output channel closed",
                        %err
                    );
                    return Err(err);
                }
            }
            stats.record("sending", start.elapsed());

            let start = time::Instant::now();
            // When no lines have been read we kick the backup_cap up by twice,
            // limited by the hard-coded cap. Else, we set the backup_cap to its
            // minimum on the assumption that next time through there will be
            // more lines to read promptly.
            backoff_cap = if global_bytes_read == 0 {
                cmp::min(2048, backoff_cap.saturating_mul(2))
            } else {
                1
            };
            let backoff = backoff_cap.saturating_sub(global_bytes_read);

            // This works only if run inside tokio context since we are using tokio's Timer.
            // Outside of such context, this will panic on the first call. Also since we are
            // using block_on here and in the above code, this should be run in its own thread.
            // `spawn_blocking` fulfills all of these requirements.
            let sleep = async move {
                if backoff > 0 {
                    tokio::time::sleep(Duration::from_millis(backoff as u64)).await;
                }
            };
            futures::pin_mut!(sleep);
            match self.handle.block_on(select(shutdown, sleep)) {
                Either::Left((_, _)) => {
                    let checkpointer = self.handle
                        .block_on(checkpoint_task_handle)
                        .expect("checkpoint task has panicked");
                    if let Err(err) = checkpointer.write_checkpoints() {
                        error!(
                            message = "Error writing checkpoints before shutdown",
                            err = ?err,
                        );
                    }

                    return Ok(Shutdown);
                }

                Either::Right((_, future)) => shutdown = future
            }

            stats.record("sleeping", start.elapsed());
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
            // not a good way to determine if they were moved or just created and written very
            // quickly, so just make sure we're not missing any data.
            ReadFrom::Beginning
        };

        // Always prefer the stored checkpoints unless the user has opted out. Previously, the
        // checkpoint was only loaded for new files when server was started up, but the
        // `kubernetes_logs` source returns the files well after start-up, once it has populated
        // them from the k8s metadata, so we now just always use the checkpoints unless opted
        // out. https://github.com/timberio/vector/issues/7139
        let read_from = if !self.ignore_checkpoints {
            checkpoints.get(fp)
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
                if let ReadFrom::Checkpoint(pos) = read_from {
                    self.emitter.emit_file_resumed(&path, pos);
                } else {
                    self.emitter.emit_file_added(&path);
                }

                watcher.set_file_findable(true);
                fps.insert(fp, watcher);
            }
            Err(err) => self.emitter.emit_file_watch_failed(&path, err)
        }
    }
}

/// A sentinel type to signal that file server was gracefully shutdown.
///
/// The purpose of this type is to clarify the semantics of the result values
/// returned from the [`Server::run`] for both the users of the file server,
/// and the implementors.
#[derive(Debug)]
pub struct Shutdown;

struct TimingStats {
    started_at: time::Instant,
    segments: BTreeMap<&'static str, Duration>,
    events: usize,
    bytes: usize,
}

impl TimingStats {
    fn record(&mut self, key: &'static str, d: Duration) {
        let segment = self.segments.entry(key).or_default();
        *segment += d;
    }

    fn record_bytes(&mut self, bytes: usize) {
        self.events += 1;
        self.bytes += bytes;
    }

    fn report(&self) {
        let total = self.started_at.elapsed();
        let counted = self.segments.values().sum();
        let other = self.started_at.elapsed() - counted;
        let mut ratios = self.segments
            .iter()
            .map((|(k, v)| (*k, v.as_secs_f32() / total.as_secs_f32())))
            .collect::<BTreeMap<_, _>>();

        ratios.insert("other", other.as_secs_f32() / total.as_secs_f32());
        let (event_throughput, bytes_throughput) = if total.as_secs() > 0 {
            (
                self.events as u64 / total.as_secs(),
                self.bytes as u64 / total.as_secs(),
            )
        } else {
            (0, 0)
        };

        debug!("event throughput {}, bytes throughput {}", scale(event_throughput), scale(bytes_throughput))
    }
}

fn scale(bytes: u64) -> String {
    let units = ["", "k", "m", "g"];
    let mut bytes = bytes as f32;
    let mut i = 0;
    while bytes > 1000.0 && i <= 3 {
        bytes /= 1000.0;
        i += 1;
    }

    format!("{:.3}{}/sec", bytes, units[i])
}

impl Default for TimingStats {
    fn default() -> Self {
        Self {
            started_at: time::Instant::now(),
            segments: Default::default(),
            events: 0,
            bytes: 0,
        }
    }
}

#[derive(Debug)]
pub struct Line {
    pub text: Bytes,
    pub filename: String,
    pub fingerprint: Fingerprint,
    pub offset: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile() {}
}