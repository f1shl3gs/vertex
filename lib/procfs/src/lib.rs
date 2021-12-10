mod arp;
mod btrfs;
mod fibre_channel;
mod nfs;
mod read;

pub(crate) use read::*;

use snafu::Snafu;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("open file {} failed, {}", path, source))]
    FileOpenFailed {
        path: String,
        source: std::io::Error,
    },

    #[snafu(display("read file {} failed, {}", path, source))]
    FileReadFailed {
        path: String,
        source: std::io::Error,
    },

    #[snafu(display("other io error, {}", source))]
    OtherErr { source: std::io::Error },
}

pub struct ProcFS {
    root: String,
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
    root: String,
}
