mod btrfs;
mod error;
mod read;

pub(crate) use read::*;

pub struct ProcFS {
    root: String
}

pub struct SysFS {
    root: String
}