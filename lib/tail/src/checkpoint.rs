use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};

const STATE_VERSION: &str = "v1";
const TMP_FILE_NAME: &str = "checkpoints.new.json";
const STABLE_FILE_NAME: &str = "checkpoints.json";

pub type Position = u64;

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
struct State {
    version: String,
    checkpoints: BTreeSet<Checkpoint>,
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct Fingerprint {
    dev: u64,
    inode: u64,
}

impl TryFrom<&PathBuf> for Fingerprint {
    type Error = std::io::Error;

    fn try_from(path: &PathBuf) -> Result<Self, Self::Error> {
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
        self.checkpoints.get(&fp).map(|r| *r.value())
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
        let to_remove = self
            .removed_times
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
        State {
            version: STATE_VERSION.to_string(),
            checkpoints: self
                .checkpoints
                .iter()
                .map(|entry| {
                    let fp = entry.key();
                    let pos = entry.value();
                    Checkpoint {
                        fingerprint: *fp,
                        position: *pos,
                        modified: self
                            .modified_times
                            .get(fp)
                            .map(|r| *r.value())
                            .unwrap_or_else(Utc::now),
                    }
                })
                .collect(),
        }
    }

    fn load(&self, checkpoint: Checkpoint) {
        self.checkpoints
            .insert(checkpoint.fingerprint, checkpoint.position);
    }

    fn set_state(&self, state: State, ignore_before: Option<DateTime<Utc>>) {
        for checkpoint in state.checkpoints {
            if let Some(ignore_before) = ignore_before {
                if checkpoint.modified < ignore_before {
                    continue;
                }
            }

            self.load(checkpoint);
        }
    }
}

pub struct Checkpointer {
    tmp_file_path: PathBuf,
    stable_file_path: PathBuf,

    checkpoints: Arc<CheckpointsView>,
    last: Mutex<Option<State>>,
}

impl Checkpointer {
    pub fn new(data_dir: &Path) -> Self {
        let tmp_file_path = data_dir.join(TMP_FILE_NAME);
        let stable_file_path = data_dir.join(STABLE_FILE_NAME);

        Checkpointer {
            tmp_file_path,
            stable_file_path,
            checkpoints: Arc::new(CheckpointsView::default()),
            last: Mutex::new(None),
        }
    }

    pub fn view(&self) -> Arc<CheckpointsView> {
        Arc::clone(&self.checkpoints)
    }

    /// Persist the current checkpoints state to disk, makeing our best effort to
    /// do so in an atomic way that allow for recovering the previous state in
    /// the event of a crash
    pub fn write_checkpoints(&self) -> Result<usize, io::Error> {
        // First drop any checkpoints for files that were removed more than 60s
        // ago. This keeps our working set as small as possible and makes sure we
        // don't spend time and IO writing checkpoints that don't matter anymore.
        self.checkpoints.remove_expired();

        let current = self.checkpoints.get_state();

        // Fetch last written state
        let mut last = self.last.lock().expect("Data poisoned");
        if last.as_ref() != Some(&current) {
            // Write the new checkpoints to a tmp file and flush it fully to
            // disk. If Vertex dies anywhere during this section, the existing
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

        Ok(self.checkpoints.checkpoints.len())
    }

    /// Read persisted checkpoints from disk
    pub fn read_checkpoints(&mut self, ignore_before: Option<DateTime<Utc>>) {
        // First try reading from the tmp file location. If this works, it means that
        // the previous process was interrupted in the process of checkpointing and
        // the temp file should contain more recent data that should be preferred.
        match self.read_checkpoints_file(&self.tmp_file_path) {
            Ok(state) => {
                info!(message = "Recovered checkpoint data from interrupted process");

                self.checkpoints.set_state(state, ignore_before);

                // Try to move this tmp file to the stable location so we don't
                // immediately overwrite it when we next persist checkpoints.
                if let Err(err) = fs::rename(&self.tmp_file_path, &self.stable_file_path) {
                    warn!(
                        message = "Error persisting recovered checkpoint file",
                        %err
                    );
                }

                return;
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                // This is expected, so no warning needed
            }
            Err(err) => {
                error!(
                    message = "Unable to recover checkpoint data from interrupted process",
                    %err
                );
            }
        }

        // Next, attempt to read checkpoints from the stable file location. This is the expected
        // location, so warn more aggressively if something goes wrong.
        match self.read_checkpoints_file(&self.stable_file_path) {
            Ok(state) => {
                info!(message = "Loaded checkpoint data",);

                self.checkpoints.set_state(state, ignore_before);
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                // This is expected, so no warning needed
            }
            Err(err) => {
                warn!(
                    message = "Unable to load checkpoint data",
                    %err
                );
            }
        }
    }

    fn read_checkpoints_file(&self, path: &Path) -> Result<State, io::Error> {
        let reader = io::BufReader::new(fs::File::open(path)?);
        serde_json::from_reader(reader)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn simple_set_and_get() {
        let fp = Fingerprint { dev: 1, inode: 2 };

        let dir = tempdir().unwrap();
        let checkpointer = Checkpointer::new(dir.path());
        let checkpoints = checkpointer.checkpoints;

        checkpoints.update(fp, 3);
        let got = checkpoints.get(Fingerprint { dev: 1, inode: 2 }).unwrap();

        assert_eq!(got, 3)
    }

    #[test]
    fn checkpointer_restart() {
        let position = 12345;
        let dir = tempdir().unwrap();
        let fp = Fingerprint { dev: 1, inode: 2 };

        {
            // checkpointer will be dropped once this block is done.
            let checkpointer = Checkpointer::new(dir.path());
            checkpointer.checkpoints.update(fp, position);
            checkpointer.write_checkpoints().unwrap();
        }

        let mut checkpointer = Checkpointer::new(dir.path());
        assert!(checkpointer.checkpoints.get(fp).is_none());
        checkpointer.read_checkpoints(None);
        assert_eq!(checkpointer.checkpoints.get(fp).unwrap(), position);
    }
}
