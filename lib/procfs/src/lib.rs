#![allow(dead_code)]

mod arp;
mod btrfs;
mod fibre_channel;
mod nfs;
mod read;

use snafu::Snafu;
use std::path::{Path, PathBuf};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Open file {} failed, {:?}", path.display(), source))]
    FileOpenFailed {
        path: PathBuf,
        source: std::io::Error,
    },

    #[snafu(display("Read file {} failed, {:?}", path.display(), source))]
    FileReadFailed {
        path: PathBuf,
        source: std::io::Error,
    },

    #[snafu(display("Other io error, {:?}", source))]
    OtherErr { source: std::io::Error },
}

pub struct ProcFS {
    root: PathBuf,
}

impl Default for ProcFS {
    fn default() -> Self {
        Self {
            root: "/proc".into(),
        }
    }
}

impl ProcFS {
    #[cfg(test)]
    pub fn test_procfs() -> Self {
        Self {
            root: "../../tests/fixtures/proc".into(),
        }
    }
}

pub struct SysFS {
    root: Path,
}
