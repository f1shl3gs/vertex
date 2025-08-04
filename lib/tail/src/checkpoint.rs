use std::collections::HashMap;
use std::fmt::Display;
use std::fs::Metadata;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

/// Header is the header of the binary file, basic information contained
///
/// ```no_run
/// struct Header {
///     // timestamp of when this binary file created, it does not necessary for us
///     timestamp: u64,
///     running: u32,
///     deleted: u32,
/// }
/// ```
const HEADER_SIZE: usize = size_of::<u64>() + 2 * size_of::<u32>();
const ENTRY_SIZE: usize = 4 * size_of::<u64>();

const DATA_FILENAME: &str = "checkpoints.data";
const TEMP_DATA_FILENAME: &str = "checkpoints.temp";

// if any entries timestamp is older than `now - EXPIRATION_IN_MILLIS`, and it doesn't
// appear in running, it should be deleted
const EXPIRATION_IN_MILLIS: u64 = 7 * 24 * 60 * 60 * 1000; // a week

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub struct Fingerprint {
    dev: u64,
    inode: u64,
}

impl Display for Fingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.dev, self.inode)
    }
}

impl From<&Metadata> for Fingerprint {
    fn from(metadata: &Metadata) -> Self {
        Fingerprint {
            dev: metadata.dev(),
            inode: metadata.ino(),
        }
    }
}

pub struct Checkpointer {
    /// The directory to store checkpoints data
    root: PathBuf,

    // The get or insert operation is not a hot path, so std::sync::Mutex
    // is good enough.
    state: Arc<Mutex<State>>,
}

impl Checkpointer {
    pub fn load(root: PathBuf) -> std::io::Result<Checkpointer> {
        if !root.exists() {
            std::fs::create_dir_all(&root)?;
        }

        let path = root.join(DATA_FILENAME);
        if !path.exists() {
            std::fs::File::create(&path)?;

            return Ok(Checkpointer {
                root,
                state: Default::default(),
            });
        }

        let buf = std::fs::read(&path)?;
        if buf.is_empty() {
            return Ok(Checkpointer {
                root,
                state: Default::default(),
            });
        }

        // read header
        if buf.len() < HEADER_SIZE {
            return Err(std::io::Error::other("data file is too short"));
        }
        // timestamp is skipped
        let running_entries = u32::from_ne_bytes(buf[8..12].try_into().unwrap());
        let deleted_entries = u32::from_ne_bytes(buf[12..16].try_into().unwrap());

        // reading running & deleted
        if buf.len() != HEADER_SIZE + ENTRY_SIZE * (running_entries + deleted_entries) as usize {
            return Err(std::io::Error::other("data file is corrupted"));
        }
        let mut pos = HEADER_SIZE;

        let mut running = HashMap::with_capacity(running_entries as usize);
        for _ in 0..running_entries {
            let dev = u64::from_ne_bytes(buf[pos..pos + 8].try_into().unwrap());
            let inode = u64::from_ne_bytes(buf[pos + 8..pos + 16].try_into().unwrap());
            let timestamp = u64::from_ne_bytes(buf[pos + 16..pos + 24].try_into().unwrap());
            let offset = u64::from_ne_bytes(buf[pos + 24..pos + 32].try_into().unwrap());

            pos += ENTRY_SIZE;

            running.insert(
                Fingerprint { dev, inode },
                (timestamp, offset, Arc::new(AtomicU64::new(offset))),
            );
        }

        let mut deleted = HashMap::with_capacity(deleted_entries as usize);
        for _ in 0..deleted_entries {
            let dev = u64::from_ne_bytes(buf[pos..pos + 8].try_into().unwrap());
            let inode = u64::from_ne_bytes(buf[pos + 8..pos + 16].try_into().unwrap());
            let timestamp = u64::from_ne_bytes(buf[pos + 16..pos + 24].try_into().unwrap());
            let offset = u64::from_ne_bytes(buf[pos + 24..pos + 32].try_into().unwrap());

            pos += ENTRY_SIZE;

            deleted.insert(
                Fingerprint { dev, inode },
                (timestamp, Arc::new(AtomicU64::new(offset))),
            );
        }

        Ok(Checkpointer {
            root,
            state: Arc::new(Mutex::new(State { running, deleted })),
        })
    }

    pub fn get(&self, fingerprint: Fingerprint) -> Option<Arc<AtomicU64>> {
        let mut state = self.state.lock().unwrap();

        if let Some((_timestamp, _last_offset, offset)) = state.running.get(&fingerprint) {
            return Some(Arc::clone(offset));
        }

        if let Some((timestamp, offset)) = state.deleted.remove(&fingerprint) {
            let last_offset = offset.load(Ordering::Acquire);

            state
                .running
                .insert(fingerprint, (timestamp, last_offset, Arc::clone(&offset)));

            return Some(offset);
        }

        None
    }

    pub fn insert(&self, fingerprint: Fingerprint, offset: u64) -> Arc<AtomicU64> {
        let mut state = self.state.lock().unwrap();

        let timestamp = match state.deleted.remove(&fingerprint) {
            Some((timestamp, _offset)) => timestamp,
            None => 0,
        };

        let ret = Arc::new(AtomicU64::new(offset));
        state
            .running
            .insert(fingerprint, (timestamp, offset, Arc::clone(&ret)));

        ret
    }

    #[inline]
    pub fn view(&self) -> CheckpointsView {
        CheckpointsView {
            state: Arc::clone(&self.state),
        }
    }

    // delete won't really delete the checkpoint, it just moves it to the `deleted` map,
    // and it will be cleared in the future.
    pub fn delete(&self, fingerprint: &Fingerprint) {
        let mut state = self.state.lock().unwrap();

        if let Some((timestamp, _last_offset, offset)) = state.running.remove(fingerprint) {
            state.deleted.insert(*fingerprint, (timestamp, offset));
        }
    }

    /// persist the checkpoints to the filesystem
    ///
    /// TODO: maybe we can avoid this function call if nothing changes
    pub fn flush(&self) -> std::io::Result<()> {
        let mut state = self.state.lock().unwrap();

        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("system clock set to invalid time")
            .as_millis() as u64;

        let mut buf = Vec::with_capacity(
            HEADER_SIZE + ENTRY_SIZE * (state.running.len() + state.deleted.len()),
        );

        // write header
        buf.extend_from_slice(&now.to_ne_bytes());
        buf.extend_from_slice(&(state.running.len() as u32).to_ne_bytes());
        buf.extend_from_slice(&(state.deleted.len() as u32).to_ne_bytes());

        state
            .running
            .iter_mut()
            .for_each(|(fingerprint, (timestamp, last_offset, offset))| {
                let offset = offset.load(Ordering::Acquire);
                let timestamp = if *last_offset == offset {
                    *timestamp
                } else {
                    *timestamp = now;
                    *last_offset = offset;
                    now
                };

                buf.extend_from_slice(&fingerprint.dev.to_ne_bytes());
                buf.extend_from_slice(&fingerprint.inode.to_ne_bytes());
                buf.extend_from_slice(&timestamp.to_ne_bytes());
                buf.extend_from_slice(&offset.to_ne_bytes());
            });

        state
            .deleted
            .retain(|_fingerprint, (timestamp, _offset)| now - *timestamp < EXPIRATION_IN_MILLIS);

        state
            .deleted
            .iter()
            .for_each(|(fingerprint, (timestamp, offset))| {
                let offset = offset.load(Ordering::Acquire);

                buf.extend_from_slice(&fingerprint.dev.to_ne_bytes());
                buf.extend_from_slice(&fingerprint.inode.to_ne_bytes());
                buf.extend_from_slice(&timestamp.to_ne_bytes());
                buf.extend_from_slice(&offset.to_ne_bytes());
            });

        let from = self.root.join(TEMP_DATA_FILENAME);
        let to = self.root.join(DATA_FILENAME);

        std::fs::write(&from, &buf)?;

        // rename is an atomic operation unless system crash
        std::fs::rename(from, to)
    }
}

#[derive(Default)]
struct State {
    running: HashMap<Fingerprint, (u64, u64, Arc<AtomicU64>)>,
    deleted: HashMap<Fingerprint, (u64, Arc<AtomicU64>)>,
}

/// A thread-safe handle for reading checkpoints in-memory across multiple threads.
pub struct CheckpointsView {
    state: Arc<Mutex<State>>,
}

impl CheckpointsView {
    pub fn get(&self, fingerprint: &Fingerprint) -> Option<Arc<AtomicU64>> {
        let state = self.state.lock().unwrap();

        if let Some((_, _, offset)) = state.running.get(fingerprint) {
            return Some(Arc::clone(offset));
        }

        if let Some((_, offset)) = state.deleted.get(fingerprint) {
            return Some(Arc::clone(offset));
        }

        None
    }
}

#[cfg(test)]
impl Checkpointer {
    fn running(&self) -> usize {
        self.state.lock().unwrap().running.len()
    }

    fn deleted(&self) -> usize {
        self.state.lock().unwrap().deleted.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use temp_dir::TempDir;

    #[test]
    fn crud() {
        let root = TempDir::new().unwrap();
        let checkpointer = Checkpointer::load(root.path().to_path_buf()).unwrap();
        assert_eq!(checkpointer.running(), 0);
        assert_eq!(checkpointer.deleted(), 0);
        checkpointer.flush().unwrap();
        drop(checkpointer);

        let checkpointer = Checkpointer::load(root.path().to_path_buf()).unwrap();
        assert_eq!(checkpointer.running(), 0);
        assert_eq!(checkpointer.deleted(), 0);

        assert!(checkpointer.get(Fingerprint { dev: 1, inode: 2 }).is_none());
        let offset = checkpointer.insert(Fingerprint { dev: 1, inode: 2 }, 2);
        assert_eq!(checkpointer.running(), 1);
        assert_eq!(checkpointer.deleted(), 0);
        assert_eq!(offset.load(Ordering::Acquire), 2);

        offset.fetch_add(3, Ordering::AcqRel);
        checkpointer.flush().unwrap();
        drop(checkpointer);

        let checkpointer = Checkpointer::load(root.path().to_path_buf()).unwrap();
        assert_eq!(checkpointer.running(), 1);
        assert_eq!(checkpointer.deleted(), 0);

        let offset = checkpointer.get(Fingerprint { dev: 1, inode: 2 }).unwrap();
        assert_eq!(checkpointer.running(), 1);
        assert_eq!(checkpointer.deleted(), 0);
        assert_eq!(offset.load(Ordering::Acquire), 5);

        checkpointer.delete(&Fingerprint { dev: 1, inode: 2 });
        checkpointer.flush().unwrap();
        drop(checkpointer);

        let checkpointer = Checkpointer::load(root.path().to_path_buf()).unwrap();
        assert_eq!(checkpointer.running(), 0);
        assert_eq!(checkpointer.deleted(), 1);
        let offset = checkpointer.get(Fingerprint { dev: 1, inode: 2 }).unwrap();
        assert_eq!(checkpointer.running(), 1);
        assert_eq!(checkpointer.deleted(), 0);
        assert_eq!(offset.load(Ordering::Acquire), 5);
    }
}
