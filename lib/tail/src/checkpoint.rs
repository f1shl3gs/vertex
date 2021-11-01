use std::io;
use std::fs;
use std::collections::BTreeSet;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};

const TMP_FILE_NAME: &str = "checkpoints.new.json";
const STABLE_FILE_NAME: &str = "checkpoints.json";

pub type Position = u64;

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "version", rename_all = "snake_case")]
enum State {
    V1 { checkpoints: BTreeSet<Checkpoint> }
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct Fingerprint {
    dev: u64,
    inode: u64,
}

impl TryFrom<PathBuf> for Fingerprint {
    type Error = std::io::Error;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let metadata = path.metadata()?;
        Ok(Self {
            dev: metadata.dev(),
            inode: metadata.ino(),
        })
    }
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Ord, PartialOrd)]
pub struct Checkpoint {
    fingerprint: Fingerprint,
    position: Position,
    modified: DateTime<Utc>,
}

/// A thread-safe handle for reading and writing checkpoints in-memory
/// access multiple threads
#[derive(Debug, Default)]
pub struct CheckpointsView {
    checkpoints: DashMap<Fingerprint, Position>,
    modified_times: DashMap<Fingerprint, DateTime<Utc>>,
    removed_times: DashMap<Fingerprint, DateTime<Utc>>,
}

impl CheckpointsView {
    pub fn update(&self, fp: Fingerprint, pos: Position) {
        self.checkpoints.insert(fp, pos);
        self.modified_times.insert(fp, Utc::now());
        self.removed_times.remove(&fp);
    }

    pub fn get(&self, fp: Fingerprint) -> Option<Position> {
        self.checkpoints
            .get(&fp)
            .map(|r| *r.value())
    }

    pub fn set_dead(&self, fp: Fingerprint) {
        self.removed_times.insert(fp, Utc::now());
    }

    pub fn update_key(&self, old: Fingerprint, new: Fingerprint) {
        if let Some((_, value)) = self.checkpoints.remove(&old) {
            self.checkpoints.insert(new, value);
        }

        if let Some((_, value)) = self.modified_times.remove(&old) {
            self.modified_times.insert(new, value);
        }

        if let Some((_, value)) = self.removed_times.remove(&old) {
            self.removed_times.insert(new, value);
        }
    }

    pub fn remove_expired(&self) {
        let now = Utc::now();

        // Collect all of the expired keys. Removing them while iterating can
        // lead to deadlocks, the set should be small, and this is not a
        // performance-sensitive path
        let to_remove = self.removed_times
            .iter()
            .filter(|entry| {
                let ts = entry.value();
                let duration = now - *ts;
                duration > chrono::Duration::seconds(60)
            })
            .map(|entry| *entry.key())
            .collect::<Vec<Fingerprint>>();

        for fp in to_remove {
            self.checkpoints.remove(&fp);
            self.modified_times.remove(&fp);
            self.removed_times.remove(&fp);
        }
    }

    fn get_state(&self) -> State {
        State::V1 {
            checkpoints: self.checkpoints
                .iter()
                .map(|entry| {
                    let fp = entry.key();
                    let pos = entry.value();
                    Checkpoint {
                        fingerprint: *fp,
                        position: *pos,
                        modified: self.modified_times
                            .get(fp)
                            .map(|r| *r.value())
                            .unwrap_or_else(Utc::now),
                    }
                })
                .collect()
        }
    }
}

pub struct Checkpointer {
    dir: PathBuf,
    tmp_file_path: PathBuf,
    stable_file_path: PathBuf,

    view: Arc<CheckpointsView>,
    last: Mutex<Option<State>>,
}

impl Checkpointer {
    pub fn new(data_dir: &Path) -> Self {
        let dir = data_dir.join("checkpoints");
        let tmp_file_path = data_dir.join(TMP_FILE_NAME);
        let stable_file_path = data_dir.join(STABLE_FILE_NAME);

        Checkpointer {
            dir,
            tmp_file_path,
            stable_file_path,
            view: Arc::new(CheckpointsView::default()),
            last: Mutex::new(None),
        }
    }

    pub fn view(&self) -> Arc<CheckpointsView> {
        Arc::clone(&self.view)
    }

    /// Persist the current checkpoints state to disk, makeing our best effort to
    /// do so in an atomic way that allow for recovering the previous state in
    /// the event of a crash
    pub fn flush(&self) -> Result<usize, io::Error> {
        // First drop any checkpoints for files that were removed more than 60s
        // ago. This keeps our working set as small as possible and makes sure we
        // don't spend time and IO writing checkpoints that don't matter anymore.
        self.view.remove_expired();

        let current = self.view.get_state();

        // Fetch last written state
        let mut last = self.last.lock().expect("Data poisoned");
        if last.as_ref() != Some(&current) {
            // Write the new checkpoints to a tmp file and flush it fully to
            // disk. If vector dies anywhere during this section, the existing
            // stable file will still be in its current valid state and we'll be
            // able to recover.
            let mut f = io::BufWriter::new(fs::File::create(&self.tmp_file_path)?);
            serde_json::to_writer(&mut f, &current)?;
            f.into_inner()?.sync_all()?;

            // Once the temp file is fully flushed, rename the tmp file to replace
            // the previous stable file. This is an atomic operation on POSIX systems
            // (and the stdlib claims to provide equivalent behavior on Windows),
            // which should prevent scenarios where we don't have at least one full
            // valid file to recover from.
            fs::rename(&self.tmp_file_path, &self.stable_file_path)?;

            *last = Some(current);
        }

        Ok(self.view.checkpoints.len())
    }
}