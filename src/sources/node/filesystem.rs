use std::{
    path::PathBuf,
    ffi::CString,
};

use crate::{
    config::{deserialize_regex, serialize_regex},
    tags,
    gauge_metric,
    event::{Metric, MetricValue},
};

use tokio::io::AsyncBufReadExt;
use serde::{Deserialize, Serialize};
use crate::sources::node::errors::{Error, ErrorContext};


#[derive(Debug, Deserialize, Serialize)]
pub struct FileSystemConfig {
    #[serde(default = "default_mount_points_exclude")]
    #[serde(deserialize_with = "deserialize_regex", serialize_with = "serialize_regex")]
    pub mount_points_exclude: regex::Regex,

    #[serde(default = "default_fs_type_exclude")]
    #[serde(deserialize_with = "deserialize_regex", serialize_with = "serialize_regex")]
    pub fs_type_exclude: regex::Regex,
}

impl Default for FileSystemConfig {
    fn default() -> Self {
        Self {
            mount_points_exclude: default_mount_points_exclude(),
            fs_type_exclude: default_fs_type_exclude(),
        }
    }
}

fn default_mount_points_exclude() -> regex::Regex {
    regex::Regex::new("^/(dev|proc|sys|var/lib/docker/.+)($|/)").unwrap()
}

fn default_fs_type_exclude() -> regex::Regex {
    regex::Regex::new(
        "^(autofs|binfmt_misc|bpf|cgroup2?|configfs|debugfs|devpts|devtmpfs|fusectl|hugetlbfs|iso9660|mqueue|nsfs|overlay|proc|procfs|pstore|rpc_pipefs|securityfs|selinuxfs|squashfs|sysfs|tracefs)$"
    ).unwrap()
}

impl FileSystemConfig {
    pub async fn gather(&self, proc_path: &str) -> Result<Vec<Metric>, Error> {
        let path = PathBuf::from(proc_path);

        let stats = self.get_stats(path).await?;
        let mut metrics = Vec::new();

        for stat in stats {
            if stat.device_error == 1 {
                metrics.push(gauge_metric!(
                    "node_filesystem_device_error",
                    "Whether an error occurred while getting statistics for the given device.",
                    1.0,
                    "device" => stat.device.clone(),
                    "mount_point" => stat.mount_point.clone(),
                    "fstype" => stat.fs_type.clone()
                ));
                continue;
            }

            metrics.push(gauge_metric!(
                "node_filesystem_size_bytes",
                "Filesystem size in bytes.",
                stat.size as f64,
                "device" => stat.device.clone(),
                "mount_point" => stat.mount_point.clone(),
                "fstype" => stat.fs_type.clone()
            ));

            metrics.push(gauge_metric!(
                "node_filesystem_free_bytes",
                "Filesystem free space in bytes.",
                stat.free as f64,
                "device" => stat.device.clone(),
                "mount_point" => stat.mount_point.clone(),
                "fstype" => stat.fs_type.clone()
            ));

            metrics.push(gauge_metric!(
                "node_filesystem_avail_bytes",
                "Filesystem space available to non-root users in bytes.",
                stat.avail as f64,
                "device" => stat.device.clone(),
                "mount_point" => stat.mount_point.clone(),
                "fstype" => stat.fs_type.clone()
            ));

            metrics.push(gauge_metric!(
                "node_filesystem_files",
                "Filesystem total file nodes.",
                stat.files as f64,
                "device" => stat.device.clone(),
                "mount_point" => stat.mount_point.clone(),
                "fstype" => stat.fs_type.clone()
            ));

            metrics.push(gauge_metric!(
                "node_filesystem_readonly",
                "Filesystem read-only status.",
                stat.ro as f64,
                "device" => stat.device.clone(),
                "mount_point" => stat.mount_point.clone(),
                "fstype" => stat.fs_type.clone()
            ));
        }

        Ok(metrics)
    }

    async fn get_stats(&self, proc_path: PathBuf) -> Result<Vec<Stat>, Error> {
        let mut path = proc_path.clone();
        path.push("mounts");

        let mut stats = Vec::new();
        let f = tokio::fs::File::open(path).await
            .context("open mounts failed")?;
        let reader = tokio::io::BufReader::new(f);
        let mut lines = reader.lines();

        while let Some(line) = lines.next_line().await? {
            let parts = line
                .split_ascii_whitespace()
                .collect::<Vec<_>>();

            if parts.len() < 4 {
                continue;
            }

            let device = parts[0].to_string();
            let mount_point = parts[1].to_string();
            let mount_point = mount_point.replace("\\040", " ");
            let mount_point = mount_point.replace("\\011", "\t");
            let fs_type = parts[2].to_string();
            let options = parts[3].to_string();

            if self.mount_points_exclude.is_match(&mount_point) {
                continue;
            }

            if self.fs_type_exclude.is_match(&fs_type) {
                continue;
            }

            let ro = options.split(',')
                .find(|&flag| flag == "ro")
                .map_or(0u64, |_| 1u64);

            match statfs(&mount_point).await {
                Ok(usage) => {
                    stats.push(Stat {
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
                    });
                }

                Err(err) => {
                    warn!(
                        "read mount point usage failed";
                        "err" => err,
                        "mount_point" => mount_point.clone(),
                    );

                    // let mount_point = mount_point.clone();
                    stats.push(Stat {
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
                    });
                }
            }
        }

        Ok(stats)
    }
}

async fn statfs(path: &str) -> Result<Usage, std::io::Error> {
    let path = CString::new(path)
        .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;

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

#[derive(Debug)]
struct Stat {
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
    async fn test_get_stats() {
        let path = PathBuf::from("testdata/proc");
        let conf = FileSystemConfig::default();
        let stats = conf.get_stats(path).await.unwrap();

        println!("{:?}", stats);

        assert_ne!(stats.len(), 0);
    }
}