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

pub struct SysFS {
    root: String,
}
