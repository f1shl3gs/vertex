use crate::fingerprinter::{Fingerprint, Fingerprinter};
use crate::Position;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};
use std::{
    fs, io,
    path::{Path, PathBuf},
};
use tracing::{error, info, warn};

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
    V1 { checkpoints: BTreeSet<Checkpoint> },
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
            self.checkpoints.insert(new, value);
        }

        if let Some((_, value)) = self.modified_times.remove(&old) {
            self.modified_times.insert(new, value);
        }

        if let Some((_, value)) = self.removed_times.remove(&old) {
            self.removed_times.insert(new, value);
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
        let to_remove = self
            .removed_times
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

                    self.load(cp);
                }
            }
        }
    }

    fn get_state(&self) -> State {
        State::V1 {
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

        if let Some((_, pos)) = self
            .checkpoints
            .remove(&Fingerprint::Unknown(fp.as_legacy()))
        {
            self.update(fp, pos);
        }

        if self.checkpoints.get(&fp).is_none() {
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
            Unknown(x) => format!("{:x}.{}", x, pos),
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
                let (dev, ino, pos) =
                    scan_fmt!(filename, "i{x}.{x}.{}", [hex u64], [hex u64], Position).unwrap();
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

    /// Write checkpoints to disk in the legacy format. Used for compatibility
    /// testing only
    #[cfg(test)]
    pub fn write_legacy_checkpoints(&mut self) -> Result<usize, io::Error> {
        fs::remove_dir_all(&self.directory).ok();
        fs::create_dir_all(&self.directory)?;
        for c in self.checkpoints.checkpoints.iter() {
            fs::File::create(self.encode(*c.key(), *c.value()))?;
        }

        Ok(self.checkpoints.checkpoints.len())
    }

    /// Read persisted checkpoints from disk, preferring the new JSON file
    /// format but falling back to the legacy system when those files are
    /// found instead.
    pub fn read_checkpoints(&mut self, ignore_before: Option<DateTime<Utc>>) {
        // First try reading from the tmp file location. If this works, it
        // means that the previous process was interrupted in the process of
        // checkpointing and the tmp file should contain more recent data
        // that should be preferred.
        match self.read_checkpoints_file(&self.tmp_file_path) {
            Ok(state) => {
                warn!("recovered checkpoint data from interrupted process");
                self.checkpoints.set_state(state, ignore_before);

                // Try to move this tmp file to the stable location so we
                // don't immediately overwrite it when we next persist
                // checkpoints
                if let Err(err) = fs::rename(&self.tmp_file_path, &self.stable_file_path) {
                    warn!("error persisting recovered checkpoint file, err: {}", err);
                }

                return;
            }

            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                // This is expected, so no warning needed
            }

            Err(err) => {
                error!(
                    "Unable to recover checkpoint data from interrupted process, err: {}",
                    err
                )
            }
        }

        // Next, attempt to read checkpoints from the stable file location. This
        // is the expected location, so warn more aggressively if something goes
        // wrong
        match self.read_checkpoints_file(&self.stable_file_path) {
            Ok(state) => {
                info!("Loaded checkpoint data");
                self.checkpoints.set_state(state, ignore_before);
                return;
            }

            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                // This is expected, so no warning need
            }

            Err(err) => {
                warn!("Unable to load checkpoint data, err: {}", err);
                return;
            }
        }

        // If we haven't returned yet, go ahead and look for the legacy files
        // and try to read them.
        info!("Attempting to read legacy checkpoint files");
        // TODO: remove this
    }

    fn read_checkpoints_file(&self, path: &Path) -> Result<State, io::Error> {
        let reader = io::BufReader::new(fs::File::open(path)?);
        serde_json::from_reader(reader)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
    }

    fn read_legacy_checkpoints(&mut self, ignore_before: Option<DateTime<Utc>>) {
        for path in glob::glob(&self.glob_string).unwrap().flatten() {
            let mut mtime = None;
            if let Some(ignore_before) = ignore_before {
                if let Ok(Ok(modified)) = fs::metadata(&path).map(|metadata| metadata.modified()) {
                    let modified = DateTime::<Utc>::from(modified);
                    if modified < ignore_before {
                        fs::remove_file(path).ok();
                        continue;
                    }

                    mtime = Some(modified)
                }
            }

            let (fp, pos) = self.decode(&path);
            self.checkpoints.checkpoints.insert(fp, pos);
            if let Some(mtime) = mtime {
                self.checkpoints.modified_times.insert(fp, mtime);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::fingerprinter::FingerprintStrategy;
    use chrono::{Duration, Utc};
    use pretty_assertions::assert_eq;
    use tempfile::tempdir;

    #[test]
    fn test_checkpointer_basics() {
        let fingerprints = vec![
            Fingerprint::DevInode(1, 2),
            Fingerprint::BytesChecksum(3456),
            Fingerprint::FirstLinesChecksum(78910),
            Fingerprint::Unknown(1337),
        ];
        for fingerprint in fingerprints {
            let position: Position = 1234;
            let data_dir = tempdir().unwrap();
            let mut chkptr = Checkpointer::new(data_dir.path());
            assert_eq!(
                chkptr.decode(&chkptr.encode(fingerprint, position)),
                (fingerprint, position)
            );
            chkptr.update_checkpoint(fingerprint, position);
            assert_eq!(chkptr.get_checkpoint(fingerprint), Some(position));
        }
    }

    #[test]
    fn test_checkpointer_ignore_before() {
        let newer = (
            Fingerprint::DevInode(1, 2),
            Utc::now() - Duration::seconds(5),
        );
        let newish = (
            Fingerprint::BytesChecksum(3456),
            Utc::now() - Duration::seconds(10),
        );
        let oldish = (
            Fingerprint::FirstLinesChecksum(78910),
            Utc::now() - Duration::seconds(15),
        );
        let older = (
            Fingerprint::Unknown(1337),
            Utc::now() - Duration::seconds(20),
        );
        let ignore_before = Some(Utc::now() - Duration::seconds(12));

        let position: Position = 1234;
        let data_dir = tempdir().unwrap();

        // load and persist the checkpoints
        {
            let chkptr = Checkpointer::new(data_dir.path());

            for (fingerprint, modified) in &[&newer, &newish, &oldish, &older] {
                chkptr.checkpoints.load(Checkpoint {
                    fingerprint: *fingerprint,
                    position,
                    modified: *modified,
                });
                assert_eq!(chkptr.get_checkpoint(*fingerprint), Some(position));
                chkptr.write_checkpoints().unwrap();
            }
        }

        // read them back and assert old are removed
        {
            let mut chkptr = Checkpointer::new(data_dir.path());
            chkptr.read_checkpoints(ignore_before);

            assert_eq!(chkptr.get_checkpoint(newish.0), Some(position));
            assert_eq!(chkptr.get_checkpoint(newer.0), Some(position));
            assert_eq!(chkptr.get_checkpoint(oldish.0), None);
            assert_eq!(chkptr.get_checkpoint(older.0), None);
        }
    }

    #[test]
    fn test_checkpointer_restart() {
        let fingerprints = vec![
            Fingerprint::DevInode(1, 2),
            Fingerprint::BytesChecksum(3456),
            Fingerprint::FirstLinesChecksum(78910),
            Fingerprint::Unknown(1337),
        ];
        for fingerprint in fingerprints {
            let position: Position = 1234;
            let data_dir = tempdir().unwrap();
            {
                let mut chkptr = Checkpointer::new(data_dir.path());
                chkptr.update_checkpoint(fingerprint, position);
                assert_eq!(chkptr.get_checkpoint(fingerprint), Some(position));
                chkptr.write_checkpoints().ok();
            }
            {
                let mut chkptr = Checkpointer::new(data_dir.path());
                assert_eq!(chkptr.get_checkpoint(fingerprint), None);
                chkptr.read_checkpoints(None);
                assert_eq!(chkptr.get_checkpoint(fingerprint), Some(position));
            }
        }
    }

    #[test]
    fn test_checkpointer_fingerprint_upgrades_unknown() {
        let log_dir = tempdir().unwrap();
        let path = log_dir.path().join("test.log");
        let data = "hello\n";
        std::fs::write(&path, data).unwrap();

        let new_fingerprint = Fingerprint::DevInode(1, 2);
        let old_fingerprint = Fingerprint::Unknown(new_fingerprint.as_legacy());
        let position: Position = 1234;
        let fingerprinter = Fingerprinter {
            strategy: FingerprintStrategy::DevInode,
            max_line_length: 1000,
            ignore_not_found: false,
        };

        let mut buf = Vec::new();

        let data_dir = tempdir().unwrap();
        {
            let mut chkptr = Checkpointer::new(data_dir.path());
            chkptr.update_checkpoint(old_fingerprint, position);
            assert_eq!(chkptr.get_checkpoint(old_fingerprint), Some(position));
            chkptr.write_checkpoints().ok();
        }
        {
            let mut chkptr = Checkpointer::new(data_dir.path());
            chkptr.read_checkpoints(None);
            assert_eq!(chkptr.get_checkpoint(new_fingerprint), None);

            chkptr.maybe_upgrade(&path, new_fingerprint, &fingerprinter, &mut buf);

            assert_eq!(chkptr.get_checkpoint(new_fingerprint), Some(position));
            assert_eq!(chkptr.get_checkpoint(old_fingerprint), None);
        }
    }

    #[test]
    fn test_checkpointer_fingerprint_upgrades_legacy_checksum() {
        let log_dir = tempdir().unwrap();
        let path = log_dir.path().join("test.log");
        let data = "hello\n";
        std::fs::write(&path, data).unwrap();

        let old_fingerprint = Fingerprint::FirstLinesChecksum(18057733963141331840);
        let new_fingerprint = Fingerprint::FirstLinesChecksum(17791311590754645022);
        let position: Position = 6;

        let fingerprinter = Fingerprinter {
            strategy: FingerprintStrategy::FirstLinesChecksum {
                ignored_header_bytes: 0,
                lines: 1,
            },
            max_line_length: 102400,
            ignore_not_found: false,
        };

        let mut buf = Vec::new();

        let data_dir = tempdir().unwrap();
        {
            let mut chkptr = Checkpointer::new(data_dir.path());
            chkptr.update_checkpoint(old_fingerprint, position);
            assert_eq!(chkptr.get_checkpoint(old_fingerprint), Some(position));
            chkptr.write_checkpoints().ok();
        }
        {
            let mut chkptr = Checkpointer::new(data_dir.path());
            chkptr.read_checkpoints(None);
            assert_eq!(chkptr.get_checkpoint(new_fingerprint), None);

            chkptr.maybe_upgrade(&path, new_fingerprint, &fingerprinter, &mut buf);

            assert_eq!(chkptr.get_checkpoint(new_fingerprint), Some(position));
            assert_eq!(chkptr.get_checkpoint(old_fingerprint), None);
        }
    }

    #[test]
    fn test_checkpointer_file_upgrades() {
        let fingerprint = Fingerprint::DevInode(1, 2);
        let position: Position = 1234;

        let data_dir = tempdir().unwrap();

        // Write out checkpoints in the legacy file format
        {
            let mut chkptr = Checkpointer::new(data_dir.path());
            chkptr.update_checkpoint(fingerprint, position);
            assert_eq!(chkptr.get_checkpoint(fingerprint), Some(position));
            chkptr.write_legacy_checkpoints().unwrap();
        }

        // Ensure that the new files were not written but the old style of files were
        assert!(!data_dir.path().join(TMP_FILE_NAME).exists());
        assert!(!data_dir.path().join(STABLE_FILE_NAME).exists());
        assert!(data_dir.path().join("checkpoints").is_dir());

        // Read from those old files, ensure the checkpoints were loaded properly, and then write
        // them normally (i.e. in the new format)
        {
            let mut chkptr = Checkpointer::new(data_dir.path());
            chkptr.read_checkpoints(None);
            assert_eq!(chkptr.get_checkpoint(fingerprint), Some(position));
            chkptr.write_checkpoints().unwrap();
        }

        // Ensure that the stable file is present, the tmp file is not, and the legacy files have
        // been cleaned up
        assert!(!data_dir.path().join(TMP_FILE_NAME).exists());
        assert!(data_dir.path().join(STABLE_FILE_NAME).exists());
        assert!(!data_dir.path().join("checkpoints").is_dir());

        // Ensure one last time that we can reread from the new files and get the same result
        {
            let mut chkptr = Checkpointer::new(data_dir.path());
            chkptr.read_checkpoints(None);
            assert_eq!(chkptr.get_checkpoint(fingerprint), Some(position));
        }
    }

    #[test]
    fn test_checkpointer_expiration() {
        let cases = vec![
            // (checkpoint, position, seconds since removed)
            (Fingerprint::BytesChecksum(123), 0, 30),
            (Fingerprint::BytesChecksum(456), 1, 60),
            (Fingerprint::BytesChecksum(789), 2, 90),
            (Fingerprint::BytesChecksum(101112), 3, 120),
        ];

        let data_dir = tempdir().unwrap();
        let mut chkptr = Checkpointer::new(data_dir.path());

        for (fingerprint, position, removed) in cases.clone() {
            chkptr.update_checkpoint(fingerprint, position);

            // slide these in manually so we don't have to sleep for a long time
            chkptr
                .checkpoints
                .removed_times
                .insert(fingerprint, Utc::now() - chrono::Duration::seconds(removed));

            assert_eq!(chkptr.get_checkpoint(fingerprint), Some(position));
        }

        // Update one that would otherwise be expired to ensure it sticks around
        chkptr.update_checkpoint(cases[2].0, 42);

        // Expiration is piggybacked on the persistence interval, so do a write to trigger it
        chkptr.write_checkpoints().unwrap();

        assert_eq!(chkptr.get_checkpoint(cases[0].0), Some(0));
        assert_eq!(chkptr.get_checkpoint(cases[1].0), None);
        assert_eq!(chkptr.get_checkpoint(cases[2].0), Some(42));
        assert_eq!(chkptr.get_checkpoint(cases[3].0), None);
    }

    #[test]
    fn test_checkpointer_checksum_updates() {
        let data_dir = tempdir().unwrap();

        let fingerprinter = Fingerprinter {
            strategy: FingerprintStrategy::Checksum {
                bytes: 16,
                ignored_header_bytes: 0,
                lines: 1,
            },
            max_line_length: 1024,
            ignore_not_found: false,
        };

        let log_path = data_dir.path().join("test.log");
        let contents = "hello i am a test log line that is just long enough but not super long\n";
        std::fs::write(&log_path, contents).expect("writing test data");

        let mut buf = vec![0; 1024];
        let old = fingerprinter
            .get_bytes_checksum(&log_path, &mut buf)
            .expect("getting old checksum")
            .expect("still getting old checksum");

        let new = fingerprinter
            .get_fingerprint_of_file(&log_path, &mut buf)
            .expect("getting new checksum");

        // make sure each is of the expected type and that the inner values are not the same
        match (old, new) {
            (Fingerprint::BytesChecksum(old), Fingerprint::FirstLinesChecksum(new)) => {
                assert_ne!(old, new)
            }
            _ => panic!("unexpected checksum types"),
        }

        let mut chkptr = Checkpointer::new(data_dir.path());

        // pretend that we had loaded this old style checksum from disk after an upgrade
        chkptr.update_checkpoint(old, 1234);

        assert!(chkptr.checkpoints.contains_bytes_checksums());

        chkptr.maybe_upgrade(&log_path, new, &fingerprinter, &mut buf);

        assert!(!chkptr.checkpoints.contains_bytes_checksums());
        assert_eq!(Some(1234), chkptr.get_checkpoint(new));
        assert_eq!(None, chkptr.get_checkpoint(old));
    }

    // guards against accidental changes to the checkpoint serialization
    #[test]
    fn test_checkpointer_serialization() {
        let fingerprints = vec![
            (
                Fingerprint::DevInode(1, 2),
                r#"{"version":"1","checkpoints":[{"fingerprint":{"dev_inode":[1,2]},"position":1234}]}"#,
            ),
            (
                Fingerprint::BytesChecksum(3456),
                r#"{"version":"1","checkpoints":[{"fingerprint":{"checksum":3456},"position":1234}]}"#,
            ),
            (
                Fingerprint::FirstLinesChecksum(78910),
                r#"{"version":"1","checkpoints":[{"fingerprint":{"first_lines_checksum":78910},"position":1234}]}"#,
            ),
            (
                Fingerprint::Unknown(1337),
                r#"{"version":"1","checkpoints":[{"fingerprint":{"unknown":1337},"position":1234}]}"#,
            ),
        ];
        for (fingerprint, expected) in fingerprints {
            let expected: serde_json::Value = serde_json::from_str(expected).unwrap();

            let position: Position = 1234;
            let data_dir = tempdir().unwrap();
            let mut chkptr = Checkpointer::new(data_dir.path());

            chkptr.update_checkpoint(fingerprint, position);
            chkptr.write_checkpoints().unwrap();

            let got: serde_json::Value = {
                let s = std::fs::read_to_string(data_dir.path().join("checkpoints.json")).unwrap();
                let mut checkpoints: serde_json::Value = serde_json::from_str(&s).unwrap();
                for checkpoint in checkpoints["checkpoints"].as_array_mut().unwrap() {
                    checkpoint.as_object_mut().unwrap().remove("modified");
                }
                checkpoints
            };

            assert_eq!(expected, got);
        }
    }

    // guards against accidental changes to the checkpoint deserialization and tests deserializing
    // old checkpoint versions
    #[test]
    fn test_checkpointer_deserialization() {
        let serialized_checkpoints = r#"
{
  "version": "1",
  "checkpoints": [
    {
      "fingerprint": { "dev_inode": [ 1, 2 ] },
      "position": 1234,
      "modified": "2021-07-12T18:19:11.769003Z"
    },
    {
      "fingerprint": { "checksum": 3456 },
      "position": 1234,
      "modified": "2021-07-12T18:19:11.769003Z"
    },
    {
      "fingerprint": { "first_line_checksum": 1234 },
      "position": 1234,
      "modified": "2021-07-12T18:19:11.769003Z"
    },
    {
      "fingerprint": { "first_lines_checksum": 78910 },
      "position": 1234,
      "modified": "2021-07-12T18:19:11.769003Z"
    },
    {
      "fingerprint": { "unknown": 1337 },
      "position": 1234,
      "modified": "2021-07-12T18:19:11.769003Z"
    }
  ]
}
        "#;
        let fingerprints = vec![
            Fingerprint::DevInode(1, 2),
            Fingerprint::BytesChecksum(3456),
            Fingerprint::FirstLinesChecksum(1234),
            Fingerprint::FirstLinesChecksum(78910),
            Fingerprint::Unknown(1337),
        ];

        let data_dir = tempdir().unwrap();

        let mut chkptr = Checkpointer::new(data_dir.path());

        std::fs::write(
            data_dir.path().join("checkpoints.json"),
            serialized_checkpoints,
        )
        .unwrap();

        chkptr.read_checkpoints(None);

        for fingerprint in fingerprints {
            assert_eq!(chkptr.get_checkpoint(fingerprint), Some(1234))
        }
    }
}
