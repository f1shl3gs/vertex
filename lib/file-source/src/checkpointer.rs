use std::collections::BTreeSet;
use crate::Position;
use std::{
    io,
    fs,
    path::{Path, PathBuf},
};
use std::sync::{Arc, Mutex};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use crate::fingerprinter::{Fingerprint, Fingerprinter};

const TMP_FILE_NAME: &str = "checkpoints.new.json";
const STABLE_FILE_NAME: &str = "checkpoints.json";

/// This enum represents the file format of checkpoints persisted to disk. Right
/// now there is only one variant, but any incompatible changes will require and
/// additional variant to be added here and handled anywhere that we transit
/// this format.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "version", rename_all = "snake_case")]
enum State {
    #[serde(rename = "1")]
    V1 {
        checkpoints: BTreeSet<Checkpoint>
    }
}

/// A simple JSON-friendly struct of the fingerprint/position pair, since
/// fingerprints as objects as objects cannot be keys in a plain JSON map
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd)]
#[serde(rename_all = "snake_case")]
struct Checkpoint {
    fingerprint: Fingerprint,
    position: Position,
    modified: DateTime<Utc>,
}

pub struct Checkpointer {
    directory: PathBuf,
    tmp_file_path: PathBuf,
    stable_file_path: PathBuf,
    glob_string: String,
    checkpoints: Arc<CheckpointsView>,
    last: Mutex<Option<State>>,
}

/// A thread-safe handle for reading and writing checkpoints in-memory
/// across multiple threads.
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
            self.checkpoints.insert(new, value)
        }

        if let Some((_, value)) = self.modified_times.remove(&old) {
            self.modified_times.insert(new, value)
        }

        if let Some((_, value)) = self.removed_times.remove(&old) {
            self.removed_times.insert(new, value)
        }
    }

    pub fn contains_bytes_checksums(&self) -> bool {
        self.checkpoints
            .iter()
            .any(|entry| matches!(entry.key(), Fingerprint::BytesChecksum(_)))
    }

    pub fn remove_expired(&self) {
        let now = Utc::now();

        // Collect all of the expired keys. Removing them whil iteration can
        // lead to deadlocks, the set should be small, and this is not a
        // performance-sensitive path.
        let to_remove = self.removed_times
            .iter()
            .filter(|entry| {
                let ts = entry.value();
                let duration = now - *ts;
                duration >= chrono::Duration::seconds(60)
            })
            .map(|entry| *entry.key())
            .collect::<Vec<Fingerprint>>();

        for fp in &to_remove {
            self.checkpoints.remove(fp);
            self.modified_times.remove(fp);
            self.removed_times.remove(fp);
        }
    }

    fn load(&self, checkpoint: Checkpoint) {
        self.checkpoints
            .insert(checkpoint.fingerprint, checkpoint.position);
        self.modified_times
            .insert(checkpoint.fingerprint, checkpoint.modified);
    }

    fn set_state(&self, state: State, ignore_before: Option<DateTime<Utc>>) {
        match state {
            State::V1 { checkpoints } => {
                for cp in checkpoints {
                    if let Some(ignore_before) = ignore_before {
                        continue;
                    }
                }

                self.load(cp);
            }
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

    fn maybe_upgrade(
        &self,
        path: &Path,
        fp: Fingerprint,
        fingerprinter: &Fingerprinter,
        buf: &mut Vec<u8>,
    ) {
        if let Ok(Some(old_checksum)) = fingerprinter.get_bytes_checksum(path, buf) {
            self.update_key(old_checksum, fp);
        }

        if let Some((_, pos)) = self.checkpoints
            .remove(&Fingerprint::Unknown(fp.as_legacy()))
        {
            self.update(fp, pos);
        }

        if self.checkpoints.get(fp).is_none() {
            if let Ok(Some(fp)) = fingerprinter.get_legacy_checksum(path, buf) {
                if let Some((_, pos)) = self.checkpoints.remove(&fp) {
                    self.update(fp, pos);
                }
            }
        }
    }
}

impl Checkpointer {
    pub fn new(root: &Path) -> Checkpointer {
        let directory = root.join("checkpoints");
        let glob_string = root.join("*").to_string_lossy().into_owned();
        let tmp_file_path = root.join(TMP_FILE_NAME);
        let stable_file_path = root.join(STABLE_FILE_NAME);

        Checkpointer {
            directory,
            glob_string,
            tmp_file_path,
            stable_file_path,
            checkpoints: Arc::new(CheckpointsView::default()),
            last: Mutex::new(None),
        }
    }

    pub fn view(&self) -> Arc<CheckpointsView> {
        Arc::clone(&self.checkpoints)
    }

    /// Encode a fingerprint to a file name, including legacy Unknown values
    ///
    /// For each of the non-legacy variants, prepend an identifier byte that
    /// falls outside of the hex range used by the legacy implementation. This
    /// allows them to be differentiated by simply peeking at the first byte.
    #[cfg(test)]
    fn encode(&self, fp: Fingerprint, pos: Position) -> PathBuf {
        use Fingerprint::*;

        let path = match fp {
            BytesChecksum(c) => format!("g{:x}.{}", c, pos),
            FirstLinesChecksum(c) => format!("h{:x}.{}", c, pos),
            DevInode(dev, ino) => format!("i{:x}.{:x}.{}", dev, ino, pos),
            Unknown(x) => format!("{:x}.{}", x, pos)
        };

        self.directory.join(path)
    }

    /// Decode a fingerprint from a file name, accounting for unknowns due to the
    /// legacy implementation
    ///
    /// The trick here is to rely on the hex encoding of the legacy format. Because
    /// hex encoding only allows [0-9a-f], we can use any character outside of that
    /// range as a magic byte identifier for the newer formats.
    fn decode(&self, path: &Path) -> (Fingerprint, Position) {
        use Fingerprint::*;

        let filename = &path.file_name().unwrap().to_string_lossy();
        match filename.chars().next().expect("empty file name") {
            'g' => {
                let (c, pos) = scan_fmt!(filename, "g{x}.{}", [hex u64], Position).unwrap();
                (BytesChecksum(c), pos)
            }

            'h' => {
                let (c, pos) = scan_fmt!(filename, "h{x}.{}", [hex u64], Position).unwrap();
                (FirstLinesChecksum(c), pos)
            }

            'i' => {
                let (dev, ino, pos) = scan_fmt!(filename, "i{x}.{x}.{}", [hex u64], [hex u64], Position).unwrap();
                (DevInode(dev, ino), pos)
            }

            _ => {
                let (c, pos) = scan_fmt!(filename, "{x}.{}", [hex u64], Position).unwrap();
                (Unknown(c), pos)
            }
        }
    }

    #[cfg(test)]
    pub fn update_checkpoint(&mut self, fp: Fingerprint, pos: Position) {
        self.checkpoints.update(fp, pos);
    }

    #[cfg(test)]
    pub fn get_checkpoint(&self, fp: Fingerprint) -> Option<Position> {
        self.checkpoints.get(fp)
    }

    // TODO: remove legacy support
    /// Scan through a given list of fresh fingerprints to see if any match
    /// an existing legacy fingerprint. If so, upgrade the existing fingerprint.
    pub fn maybe_upgrade(
        &mut self,
        path: &Path,
        fresh: Fingerprint,
        fingerprinter: &Fingerprinter,
        buf: &mut Vec<u8>,
    ) {
        self.checkpoints
            .maybe_upgrade(path, fresh, fingerprinter, buf)
    }

    /// Persist the current checkpoints state to disk, making our best effort to
    /// do so in an atomic way that allow for recovering the previous state in
    /// the event of a crash
    pub fn write_checkpoints(&self) -> Result<usize, io::Error> {
        // First drop any checkpoints for files that were removed more than 60
        // seconds ago. This keeps our woking set as small as possible and
        // makes sure we don't spend time and IO writing checkpoints that don't
        // matter anymore
        self.checkpoints.remove_expired();

        let current = self.checkpoints.get_state();

        // Fetch last written state
        let mut last = self.last.lock().expect("Data poisoned");
        if last.as_ref() != Some(&current) {
            // Write the new checkpoints to a tmp file and flush it fully to
            // disk. If vertex dies anywhere during this section, the existing
            // stable file will still be in its current valid state and we'll be
            // able to recover
            let mut f = io::BufWriter::new(fs::File::create(&self.tmp_file_path)?);
            serde_json::to_writer(&mut f, &current)?;
            f.into_inner()?.sync_all()?;

            // Once the temp file is fully flushed, rename the tmp file to replace
            // the previous stable file. This is an atomic operation on POSIX
            // systems (and the stdlib claims to provide equivalent behavior on
            // Windows), which should prevent scenarios where we don't have at least
            // one full valid file to recover from.
            fs::rename(&self.tmp_file_path, &self.stable_file_path)?;

            *last = Some(current)
        }

        Ok(self.checkpoints.checkpoints.len())
    }
}