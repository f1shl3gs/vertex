use std::collections::BTreeSet;
use crate::FilePosition;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use dashmap::DashMap;

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
    fingerprint: FileFingerprint,
    position: FilePosition,
    modified: DateTime<Utc>,
}

pub struct Checkpointer {
    directory: PathBuf,
    tmp_file_path: PathBuf,
    stable_file_path: PathBuf,
    glob_string: String,
    checkpoints: Arc<CheckpointsView>,
    last: Mutex<Option<State>>
}

/// A thread-safe handle for reading and writing checkpoints in-memory
/// across multiple threads.
#[derive(Debug, Default)]
pub struct CheckpointsView {
    checkpoints: DashMap<FileFingerprint, FilePosition>,
    modified_times: DashMap<FileFingerprint, DateTime<Utc>>,
    removed_times: DashMap<FileFingerprint, DateTime<Utc>>,
}

impl CheckpointsView {
    pub fn update(&self, fp: FileFingerprint, pos: FilePosition) {

    }
}