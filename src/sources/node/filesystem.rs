use std::ffi::CString;
use std::path::PathBuf;

use configurable::Configurable;
use event::{Metric, tags, tags::Key};
use framework::config::serde_regex;
use serde::{Deserialize, Serialize};

use super::Error;

#[derive(Clone, Configurable, Debug, Deserialize, Serialize)]
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
        "^(autofs|binfmt_misc|bpf|cgroup2?|configfs|debugfs|devpts|devtmpfs|fusectl|hugetlbfs|iso9660|mqueue|nsfs|overlay|proc|procfs|pstore|rpc_pipefs|securityfs|selinuxfs|squashfs|erofs|sysfs|tracefs)$"
    ).unwrap()
}

pub async fn gather(conf: Config, proc_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let stats = get_stats(&conf, proc_path)?;

    let mut metrics = Vec::with_capacity(stats.len() * 8);
    for stat in stats {
        let device_error = stat.device_error.is_some();
        let tags = tags!(
            Key::from_static("device") => stat.device.clone(),
            Key::from_static("fstype") => stat.fs_type,
            Key::from_static("mountpoint") => stat.mount_point.clone(),
            Key::from_static("device_error") => stat.device_error.unwrap_or_default(),
        );

        metrics.extend([
            Metric::gauge_with_tags(
                "node_filesystem_device_error",
                "Whether an error occurred while getting statistics for the given device.",
                device_error,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "node_filesystem_readonly",
                "Filesystem read-only status.",
                stat.ro,
                tags.clone(),
            ),
        ]);

        if device_error {
            continue;
        }

        metrics.extend([
            Metric::gauge_with_tags(
                "node_filesystem_size_bytes",
                "Filesystem size in bytes.",
                stat.size,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "node_filesystem_free_bytes",
                "Filesystem free space in bytes.",
                stat.free,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "node_filesystem_avail_bytes",
                "Filesystem space available to non-root users in bytes.",
                stat.avail,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "node_filesystem_files",
                "Filesystem total file nodes.",
                stat.files,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "node_filesystem_files_free",
                "Filesystem total free file nodes",
                stat.files_free,
                tags,
            ),
            Metric::gauge_with_tags(
                "node_filesystem_mount_info",
                "Filesystem mount information",
                1,
                tags!(
                    "device" => stat.device,
                    "major" => stat.major,
                    "minor" => stat.minor,
                    "mountpoint" => stat.mount_point,
                ),
            ),
        ]);
    }

    Ok(metrics)
}

fn get_stats(config: &Config, root: PathBuf) -> Result<Vec<Stat>, Error> {
    let data = std::fs::read_to_string(root.join("1/mountinfo"))
        .or_else(|_err| std::fs::read_to_string(root.join("self/mountinfo")))?;

    let mut stats = Vec::new();
    for line in data.lines() {
        let parts = line.split_ascii_whitespace().collect::<Vec<_>>();
        if parts.len() < 10 {
            return Err(Error::Other(format!(
                "malformed mount point information: {line}"
            )));
        }

        let mut m = 5;
        while parts[m + 1] != "-" {
            m += 1;
        }

        // Ensure we handle the translation of \040 and \011
        // as per fstab(5)
        let mount_point = parts[4].replace("\\040", " ").replace("\\011", "\t");
        if config.mount_points_exclude.is_match(&mount_point) {
            continue;
        }

        let fs_type = parts[m + 2];
        if config.fs_type_exclude.is_match(fs_type) {
            continue;
        }

        let device = parts[m + 3];
        let options = parts[5];
        let ro = options
            .split(',')
            .find(|&flag| flag == "ro")
            .map_or(0u64, |_| 1u64);

        let (major, minor) = parts[2]
            .split_once(':')
            .map(|(major, minor)| (major.to_string(), minor.to_string()))
            .unwrap_or_default();

        match statfs(&mount_point) {
            Ok(usage) => {
                stats.push(Stat {
                    device: device.to_string(),
                    mount_point: mount_point.clone(),
                    fs_type: fs_type.to_string(),
                    options: options.to_string(),
                    ro,
                    size: usage.size(),
                    free: usage.free(),
                    avail: usage.avail(),
                    files: usage.files(),
                    files_free: usage.files_free(),
                    device_error: None,
                    major,
                    minor,
                });
            }

            Err(err) => {
                debug!(
                    message = "read mount point usage failed",
                    %err,
                    %mount_point,
                );

                stats.push(Stat {
                    device: device.to_string(),
                    fs_type: fs_type.to_string(),
                    options: options.to_string(),
                    mount_point: mount_point.clone(),
                    size: 0,
                    free: 0,
                    avail: 0,
                    files: 0,
                    files_free: 0,
                    ro: 0,
                    device_error: Some(err.kind().to_string()),
                    major,
                    minor,
                });
            }
        }
    }

    Ok(stats)
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

#[derive(Debug)]
struct Stat {
    device: String,
    mount_point: String,
    fs_type: String,
    options: String,
    major: String,
    minor: String,

    size: u64,
    free: u64,
    avail: u64,
    files: u64,
    files_free: u64,
    ro: u64,
    device_error: Option<String>,
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

    #[test]
    fn test_get_stats() {
        let path = PathBuf::from("tests/node/proc");
        let conf = Config::default();
        let stats = get_stats(&conf, path).unwrap();
        assert_ne!(stats.len(), 0);
    }
}
