#![allow(dead_code)]

mod arp;
mod bcache;
mod bonding;
mod btrfs;
mod conntrack;
mod cpu;
mod cpufreq;
mod diskstats;
mod drm;
mod edac;
mod entropy;
mod fibrechannel;
mod filefd;
mod filesystem;
mod nfs;

use glob::{GlobError, PatternError};
use std::io::Read;
use std::num::ParseIntError;
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(Debug)]
pub enum Error {
    Io { err: std::io::Error },
    InvalidData { err: std::io::Error },
    ParseInteger { err: ParseIntError },
    Glob { err: PatternError },
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Io { err }
    }
}

impl From<glob::PatternError> for Error {
    fn from(err: PatternError) -> Self {
        Self::Glob { err }
    }
}

impl From<glob::GlobError> for Error {
    fn from(_: GlobError) -> Self {
        todo!()
    }
}

impl From<ParseIntError> for Error {
    fn from(err: ParseIntError) -> Self {
        Self::ParseInteger { err }
    }
}

impl Error {
    pub fn invalid_data<E>(err: E) -> Self
    where
        E: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        let err = std::io::Error::new(std::io::ErrorKind::InvalidData, err);
        Self::InvalidData { err }
    }
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
            root: "fixtures/proc".into(),
        }
    }
}

pub struct SysFS {
    root: PathBuf,
}

impl SysFS {
    #[cfg(test)]
    pub fn test_sysfs() -> Self {
        Self {
            root: "fixtures/sys".into(),
        }
    }
}

/// `read_to_string` should be a async function, but the implement do sync calls from
/// std, which will not call spawn_blocking and create extra threads for IO reading. It
/// actually reduce cpu usage an memory. The `tokio-uring` should be introduce once it's
/// ready.
///
/// The files this function will(should) be reading is under `/sys` and `/proc` which is
/// relative small and the filesystem is kind of `tmpfs`, so the performance should never
/// be a problem.
pub async fn read_to_string<P: AsRef<Path>>(path: P) -> Result<String, std::io::Error> {
    let mut file = std::fs::File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    Ok(content.trim_end().to_string())
}

pub async fn read_into<P, T, E>(path: P) -> Result<T, Error>
where
    P: AsRef<Path>,
    T: FromStr<Err = E>,
    Error: From<E>,
{
    let content = read_to_string(path).await?;
    Ok(<T as FromStr>::from_str(content.as_str())?)
}
