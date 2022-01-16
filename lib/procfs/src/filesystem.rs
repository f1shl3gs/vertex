use std::ffi::CString;
use tokio::io::AsyncBufReadExt;

use crate::{Error, ProcFS};

#[derive(Debug)]
pub struct FsStat {
    device: String,
    mount_point: String,
    fs_type: String,
    options: String,

    size: u64,
    free: u64,
    avail: u64,
    files: u64,
    files_free: u64,
    ro: u64,
    device_error: u64,
}

impl ProcFS {
    pub async fn filesystem(&self) -> Result<Vec<FsStat>, Error> {
        let path = self.root.join("mounts");
        let f = tokio::fs::File::open(path).await?;
        let reader = tokio::io::BufReader::new(f);
        let mut lines = reader.lines();
        let mut tasks = vec![];

        while let Some(line) = lines.next_line().await? {
            let parts = line.split_ascii_whitespace().collect::<Vec<_>>();

            if parts.len() < 4 {
                continue;
            }

            let device = parts[0].to_string();
            let mount_point = parts[1].to_string();
            let mount_point = mount_point.replace("\\040", " ");
            let mount_point = mount_point.replace("\\011", "\t");
            let fs_type = parts[2].to_string();
            let options = parts[3].to_string();

            let ro = options
                .split(',')
                .find(|&flag| flag == "ro")
                .map_or(0u64, |_| 1u64);

            let handler = tokio::task::spawn(async move {
                match statfs(&mount_point).await {
                    Ok(usage) => FsStat {
                        device,
                        mount_point: mount_point.clone(),
                        fs_type,
                        options,
                        ro,
                        size: usage.size(),
                        free: usage.free(),
                        avail: usage.avail(),
                        files: usage.files(),
                        files_free: usage.files_free(),
                        device_error: 0,
                    },

                    Err(_err) => FsStat {
                        device,
                        fs_type,
                        options,
                        mount_point: mount_point.clone(),
                        size: 0,
                        free: 0,
                        avail: 0,
                        files: 0,
                        files_free: 0,
                        ro: 0,
                        device_error: 1,
                    },
                }
            });

            tasks.push(handler);
        }

        let stats = futures::future::join_all(tasks)
            .await
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        Ok(stats)
    }
}

async fn statfs(path: &str) -> Result<Usage, std::io::Error> {
    let path =
        CString::new(path).map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;

    let mut vfs = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    let result = unsafe { libc::statvfs(path.as_ptr(), vfs.as_mut_ptr()) };

    if result == 0 {
        let vfs = unsafe { vfs.assume_init() };
        Ok(Usage(vfs))
    } else {
        // Err(std::error::Error::last_os_error().with_ffi("statvfs"))
        Err(std::io::Error::last_os_error())
    }
}

struct Usage(libc::statvfs);

impl Usage {
    #[inline]
    pub fn size(&self) -> u64 {
        self.0.f_blocks * self.0.f_frsize
    }

    #[inline]
    fn free(&self) -> u64 {
        self.0.f_bfree * self.0.f_bsize
    }

    #[inline]
    fn avail(&self) -> u64 {
        self.0.f_bavail * self.0.f_bsize
    }

    #[inline]
    fn files(&self) -> u64 {
        self.0.f_files
    }

    #[inline]
    fn files_free(&self) -> u64 {
        self.0.f_ffree
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_filesystem() {
        let procfs = ProcFS::test_procfs();
        let stats = procfs.filesystem().await.unwrap();

        println!("ss: {:#?}", stats);
    }
}
