use std::path::Path;

use event::Metric;

const USER_HZ: f64 = 100.0;
const PAGE_SIZE: f64 = 4096.0;

pub fn proc_info(root: &Path) -> Vec<Metric> {
    let pid = std::process::id();
    let mut metrics = Vec::with_capacity(10);

    match get_proc_stat(root, pid) {
        Ok((cpu_total, threads, start_time, vsize, rss)) => metrics.extend([
            Metric::sum(
                "process_cpu_seconds_total",
                "Total user and system CPU time spent in seconds",
                cpu_total,
            ),
            Metric::gauge("process_threads", "Number of OS threads created", threads),
            Metric::gauge(
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
        ]),
        Err(err) => {
            warn!(
                message = "get process stats failed",
                %err
            );
        }
    };

    match open_fds(pid) {
        Ok(fds) => {
            metrics.push(Metric::gauge(
                "process_open_fds",
                "Number of open file descriptors",
                fds,
            ));
        }
        Err(err) => {
            warn!(
                message = "read open fd files failed",
                %err
            )
        }
    }

    match get_limits(pid) {
        Ok((max_fds, max_vss)) => metrics.extend([
            Metric::gauge(
                "process_max_fds",
                "Maximum number of open file descriptors.",
                max_fds,
            ),
            Metric::gauge(
                "process_virtual_memory_max_bytes",
                "Maximum amount of virtual memory available in bytes, 0 for unlimited",
                max_vss,
            ),
        ]),
        Err(err) => {
            warn!(
                message = "read process limits failed",
                %err
            );
        }
    }

    // Only on systems with procfs, collect the count of bytes received/transmitted
    // of the process. But error will not be returned even if procfs not enabled.
    //
    // https://github.com/prometheus/client_golang/pull/1555
    match get_self_netstat(root, pid) {
        Ok((received_bytes, transmit_bytes)) => metrics.extend([
            Metric::sum(
                "process_network_receive_bytes_total",
                "Number of bytes received by the process over the network.",
                received_bytes,
            ),
            Metric::sum(
                "process_network_transmit_bytes_total",
                "Number of bytes sent by the process over the network.",
                transmit_bytes,
            ),
        ]),
        Err(err) => {
            warn!(
                message = "read network tx/rx failed",
                %err
            )
        }
    }

    metrics
}

fn open_fds(pid: u32) -> Result<usize, std::io::Error> {
    let path = format!("/proc/{}/fd", pid);

    std::fs::read_dir(path)?.try_fold(0usize, |acc, item| {
        let entry = item?;
        let ty = entry.file_type()?;
        let next = if !ty.is_dir() { acc + 1 } else { acc };

        Ok(next)
    })
}

fn get_limits(pid: u32) -> Result<(f64, f64), std::io::Error> {
    let path = format!("/proc/{}/limits", pid);
    let data = std::fs::read_to_string(path)?;

    let mut max_fds = 0.0;
    let mut max_vss = 0.0;
    for line in data.lines() {
        if let Some(s) = line.strip_prefix("Max open files") {
            let fields = s.split_whitespace().collect::<Vec<_>>();
            max_fds = fields[1].parse().map_err(|_err| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "parse `Max open files` failed",
                )
            })?;
            continue;
        }

        if let Some(s) = line.strip_prefix("Max address space") {
            let fields = s.split_whitespace().collect::<Vec<_>>();
            if fields[1] == "unlimited" {
                continue;
            }

            max_vss = fields[1].parse().map_err(|_err| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "parse `Max address space` failed",
                )
            })?;
            continue;
        }
    }

    Ok((max_fds, max_vss))
}

fn get_boot_time(root: &Path) -> Result<f64, std::io::Error> {
    let data = std::fs::read_to_string(root.join("stat"))?;
    for line in data.lines() {
        if let Some(value) = line.strip_prefix("btime ") {
            let value = value
                .parse::<f64>()
                .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
            return Ok(value);
        }
    }

    Err(std::io::Error::from(std::io::ErrorKind::NotFound))
}

fn get_proc_stat(root: &Path, pid: u32) -> Result<(f64, f64, f64, f64, f64), std::io::Error> {
    let path = root.join(pid.to_string()).join("stat");
    let content = std::fs::read_to_string(path)?;
    let parts = content.split_ascii_whitespace().collect::<Vec<_>>();

    let btime = get_boot_time(root)?;
    let utime = parts[13].parse().unwrap_or(0f64);
    let stime = parts[14].parse().unwrap_or(0f64);
    let threads = parts[19].parse().unwrap_or(0f64);
    let start_time = parts[21].parse().unwrap_or(0f64);
    let vsize = parts[22].parse().unwrap_or(0f64);
    let rss = parts[23].parse().unwrap_or(0f64);

    Ok((
        (utime + stime) / USER_HZ,
        threads,
        btime + (start_time) / USER_HZ,
        vsize,
        rss,
    ))
}

fn get_self_netstat(root: &Path, pid: u32) -> Result<(f64, f64), std::io::Error> {
    let path = root.join(pid.to_string()).join("net/netstat");
    let data = std::fs::read_to_string(path)?;

    let mut lines = data.lines();
    let (keys, values) = loop {
        let line = lines
            .next()
            .ok_or(std::io::Error::from(std::io::ErrorKind::UnexpectedEof))?;
        if !line.starts_with("IpExt: ") {
            continue;
        }

        let column_names = line.split_whitespace().collect::<Vec<_>>();
        let value_line = lines
            .next()
            .ok_or(std::io::Error::from(std::io::ErrorKind::UnexpectedEof))?;
        let values = value_line.split_whitespace().collect::<Vec<_>>();

        break (column_names, values);
    };

    let in_octets = keys
        .iter()
        .position(|n| *n == "InOctets")
        .map(|index| values[index].parse::<f64>())
        .transpose()
        .map_err(|_err| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "parse `InOctets` failed")
        })?
        .ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "`InOctets` not found`")
        })?;
    let out_octets = keys
        .iter()
        .position(|n| *n == "OutOctets")
        .map(|index| values[index].parse::<f64>())
        .transpose()
        .map_err(|_err| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "parse `OutOctets` failed")
        })?
        .ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "`OutOctets` not found`")
        })?;

    Ok((in_octets, out_octets))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_stat() {
        let root = Path::new("tests/node/proc");
        let (cpu_time, threads, _, vsize, rss) = get_proc_stat(root, 26231).unwrap();

        assert_eq!(cpu_time, 17.21);
        assert_eq!(threads, 1.0);
        assert_eq!(vsize, 56274944.0);
        assert_eq!(rss, 1981.0);
    }
}
