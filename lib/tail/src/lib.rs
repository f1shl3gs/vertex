#![allow(async_fn_in_trait)]

mod checkpoint;
pub mod decode;
mod harvest;
mod multiline;
mod notify;
mod ready_frames;

pub use checkpoint::{Checkpointer, CheckpointsView, Fingerprint};
pub use harvest::harvest;
pub use multiline::{Logic, Multiline};
pub use notify::FileReader;
pub use ready_frames::ReadyFrames;

use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

pub type Shutdown = tokio::sync::oneshot::Receiver<()>;

#[derive(Copy, Clone, Debug, Default)]
pub enum ReadFrom {
    Beginning,

    #[default]
    End,
}

pub trait Provider {
    type Metadata: Debug;

    // maybe provider should not return rotated files, since we don't really need
    // it unless we are catching up
    async fn scan(&mut self) -> std::io::Result<Vec<(PathBuf, Self::Metadata)>>;
}

pub trait Conveyor {
    type Metadata: Debug;

    /// This function used to read FileReader and send it other sinks, caller must
    /// track the offset and handle shutdown properly, otherwise harvest will not
    /// start/end properly.
    fn run(
        &self,
        reader: FileReader,
        meta: Self::Metadata,
        offset: Arc<AtomicU64>,
        shutdown: Shutdown,
    ) -> impl Future<Output = Result<(), ()>> + Send + 'static;
}
