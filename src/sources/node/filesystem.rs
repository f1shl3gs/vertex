use std::path::PathBuf;
use std::{ffi::CString, path::Path};

use event::{tags, tags::Key, Metric};
use framework::config::serde_regex;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncBufReadExt;

use super::Error;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    #[serde(default = "default_mount_points_exclude")]
    #[serde(with = "serde_regex")]
    mount_points_exclude: regex::Regex,

    #[serde(default = "default_fs_type_exclude")]
    #[serde(with = "serde_regex")]
    fs_type_exclude: regex::Regex,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mount_points_exclude: default_mount_points_exclude(),
            fs_type_exclude: default_fs_type_exclude(),
        }
    }
}

fn default_mount_points_exclude() -> regex::Regex {
    regex::Regex::new(
        "^/(dev|proc|run/credentials/.+|sys|var/lib/docker/.+|var/lib/containers/storage/.+)($|/)",
    )
    .unwrap()
}

fn default_fs_type_exclude() -> regex::Regex {
    regex::Regex::new(
        "^(autofs|binfmt_misc|bpf|cgroup2?|configfs|debugfs|devpts|devtmpfs|fusectl|hugetlbfs|iso9660|mqueue|nsfs|overlay|proc|procfs|pstore|rpc_pipefs|securityfs|selinuxfs|squashfs|sysfs|tracefs)$"
    ).unwrap()
}

pub async fn gather(conf: Config, proc_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let stats = conf.get_stats(proc_path.join("mounts")).await?;

    let mut metrics = Vec::new();
    for stat in stats {
        let tags = tags!(
            Key::from_static("device") => stat.device,
            Key::from_static("fstype") => stat.fs_type,
            Key::from_static("mount_point") => stat.mount_point,
        );

        if stat.device_error == 1 {
            metrics.push(Metric::gauge_with_tags(
                "node_filesystem_device_error",
                "Whether an error occurred while getting statistics for the given device.",
                1.0,
                tags,
            ));
            continue;
        }

        metrics.extend([
            Metric::gauge_with_tags(
                "node_filesystem_size_bytes",
                "Filesystem size in bytes.",
                stat.size as f64,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "node_filesystem_free_bytes",
                "Filesystem free space in bytes.",
                stat.free as f64,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "node_filesystem_avail_bytes",
                "Filesystem space available to non-root users in bytes.",
                stat.avail as f64,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "node_filesystem_files",
                "Filesystem total file nodes.",
                stat.files as f64,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "node_filesystem_readonly",
                "Filesystem read-only status.",
                stat.ro as f64,
                tags,
            ),
        ]);
    }

    Ok(metrics)
}

impl Config {
    async fn get_stats<P: AsRef<Path>>(&self, path: P) -> Result<Vec<Stat>, Error> {
        let mut stats = Vec::new();
        let f = tokio::fs::File::open(path).await?;
        let reader = tokio::io::BufReader::new(f);
        let mut lines = reader.lines();

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

            if self.mount_points_exclude.is_match(&mount_point) {
                continue;
            }

            if self.fs_type_exclude.is_match(&fs_type) {
                continue;
            }

            let ro = options
                .split(',')
                .find(|&flag| flag == "ro")
                .map_or(0u64, |_| 1u64);

            match statfs(&mount_point) {
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
                    debug!(
                        message = "read mount point usage failed",
                        %err,
                        %mount_point,
                    );

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

fn statfs(path: &str) -> Result<Usage, std::io::Error> {
    let path =
        CString::new(path).map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;

    let mut vfs = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    let result = unsafe { libc::statvfs(path.as_ptr(), vfs.as_mut_ptr()) };

    if result == 0 {
        let vfs = unsafe { vfs.assume_init() };
        Ok(Usage(vfs))
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[allow(dead_code)]
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
    use std::path::PathBuf;

    use super::*;

    #[tokio::test]
    async fn test_get_stats() {
        let path = PathBuf::from("tests/fixtures/proc/mounts");
        let conf = Config::default();
        let stats = conf.get_stats(path).await.unwrap();
        assert_ne!(stats.len(), 0);
    }
}
