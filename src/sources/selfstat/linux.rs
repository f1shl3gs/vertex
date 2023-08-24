use std::io::Read;

use event::Metric;

const USER_HZ: f64 = 100.0;
const PAGE_SIZE: f64 = 4096.0;

pub async fn proc_info() -> Result<Vec<Metric>, std::io::Error> {
    let pid = unsafe { libc::getpid() };
    let fds = open_fds(pid)? as f64;
    let max_fds = max_fds(pid)?;
    let (cpu_total, threads, start_time, vsize, rss) = get_proc_stat("/proc", pid).await?;

    Ok(vec![
        Metric::gauge(
            "process_max_fds",
            "Maximum number of open file descriptors.",
            max_fds,
        ),
        Metric::gauge("process_open_fds", "Number of open file descriptors", fds),
        Metric::sum(
            "process_cpu_seconds_total",
            "Total user and system CPU time spent in seconds",
            cpu_total,
        ),
        Metric::sum(
            "process_start_time_seconds",
            "Start time of the process since unix epoch in seconds",
            start_time,
        ),
        Metric::gauge(
            "process_virtual_memory_bytes",
            "Virtual memory size in bytes",
            vsize,
        ),
        Metric::gauge(
            "process_resident_memory_bytes",
            "Resident memory size in bytes",
            rss * PAGE_SIZE,
        ),
        Metric::gauge("process_threads", "Number of OS threads created", threads),
    ])
}

fn open_fds(pid: i32) -> Result<usize, std::io::Error> {
    let path = format!("/proc/{}/fd", pid);

    std::fs::read_dir(path)?.try_fold(0usize, |acc, item| {
        let entry = item?;
        let ty = entry.file_type()?;
        let next = if !ty.is_dir() { acc + 1 } else { acc };

        Ok(next)
    })
}

const MAXFD_PATTERN: &str = "Max open files";

fn max_fds(pid: i32) -> Result<f64, std::io::Error> {
    let mut buffer = String::new();
    std::fs::File::open(format!("/proc/{}/limits", pid))
        .and_then(|mut f| f.read_to_string(&mut buffer))?;

    find_statistic(&buffer, MAXFD_PATTERN)
}

fn find_statistic(all: &str, pat: &str) -> Result<f64, std::io::Error> {
    if let Some(idx) = all.find(pat) {
        let mut iter = (all[idx + pat.len()..]).split_whitespace();
        if let Some(v) = iter.next() {
            return v.parse().map_err(|e| {
                // Error::Msg(format!("read statistic {} failed: {}", pat, e))
                std::io::Error::new(std::io::ErrorKind::InvalidInput, e)
            });
        }
    }

    Err(std::io::Error::from(std::io::ErrorKind::InvalidInput))
}

async fn get_proc_stat(root: &str, pid: i32) -> Result<(f64, f64, f64, f64, f64), std::io::Error> {
    let path = format!("{}/{}/stat", root, pid);
    let content = tokio::fs::read_to_string(&path).await?;
    let parts = content.split_ascii_whitespace().collect::<Vec<_>>();

    let utime = parts[13].parse().unwrap_or(0f64);
    let stime = parts[14].parse().unwrap_or(0f64);
    let threads = parts[19].parse().unwrap_or(0f64);
    let start_time = parts[21].parse().unwrap_or(0f64);
    let vsize = parts[22].parse().unwrap_or(0f64);
    let rss = parts[23].parse().unwrap_or(0f64);

    Ok((
        (utime + stime) / USER_HZ,
        threads,
        (start_time) / USER_HZ,
        vsize,
        rss,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_proc_stat() {
        let (cpu_time, threads, _, vsize, rss) =
            get_proc_stat("tests/fixtures/proc", 26231).await.unwrap();

        assert_eq!(cpu_time, 17.21);
        assert_eq!(threads, 1.0);
        assert_eq!(vsize, 56274944.0);
        assert_eq!(rss, 1981.0);
    }
}
