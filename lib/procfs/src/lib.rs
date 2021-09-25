mod btrfs;
mod error;
mod read;
mod fibre_channel;
mod nfs;

pub(crate) use read::*;

pub struct ProcFS {
    root: String
}

pub struct SysFS {
    root: String
}