use std::path::PathBuf;

use event::Metric;

use super::{Error, read_into};

pub async fn gather(proc_path: PathBuf, sys_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let enabled = get_enabled(proc_path)?;
    let mut metrics = vec![Metric::gauge(
        "node_selinux_enabled",
        "SELinux is enabled, 1 is true, 0 is false",
        enabled,
    )];

    if enabled {
        metrics.extend([
            Metric::gauge(
                "node_selinux_config_mode",
                "Configured SELinux enforcement mode",
                default_enforce_mode()?,
            ),
            Metric::gauge(
                "node_selinux_current_mode",
                "Current SELinux enforcement mode",
                read_into::<_, i32, _>(sys_path.join("fs/selinux/enforce"))?,
            ),
        ]);
    }

    Ok(metrics)
}

fn get_enabled(proc_path: PathBuf) -> Result<bool, Error> {
    let thread_self = proc_path.join("thread-self/attr/current");
    let path = if thread_self.exists() {
        // Linux >= 3.17 provides this
        thread_self
    } else {
        let thread_id = unsafe { libc::syscall(libc::SYS_gettid) as i64 };
        proc_path.join(format!("self/task/{thread_id}/attr/current"))
    };

    // The content is end with '0x0000'
    let content = std::fs::read_to_string(path)?;

    Ok(!content.starts_with("kernel"))
}

fn default_enforce_mode() -> Result<bool, Error> {
    let data = std::fs::read_to_string("/etc/selinux/config")?;

    for line in data.lines() {
        if let Some(value) = line.strip_prefix("SELINUX=") {
            return if value == "enforcing" {
                Ok(true)
            } else if value == "permissive" {
                Ok(false)
            } else {
                Err(format!("unknown enforce mode \"{value}\"").into())
            };
        }
    }

    Ok(false)
}
