mod buffer;
mod checkpoint;
mod harvester;
pub mod provider;
mod watch;

// re-export
pub use buffer::*;
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
use crate::watch::Watcher;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ReadFrom {
    Beginning,
    End,
    Checkpoint(Position),
}
