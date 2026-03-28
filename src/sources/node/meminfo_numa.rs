use std::borrow::Cow;

use event::{Metric, tags};

use super::{Error, Paths, read_file_no_stat};

pub async fn collect(paths: Paths) -> Result<Vec<Metric>, Error> {
    let root = paths.sys().join("devices/system/node");

    let mut metrics = Vec::new();
    for entry in root.read_dir()?.flatten() {
        let filename = entry.file_name();
        let filename = filename.to_string_lossy();
        let Some(stripped) = filename.strip_prefix("node") else {
            continue;
        };
        let Ok(numa) = stripped.parse::<u64>() else {
            continue;
        };

        let content = read_file_no_stat(entry.path().join("meminfo"))?;
        for info in parse_meminfo_numa(&content)? {
            metrics.push(Metric::gauge_with_tags(
                format!("node_memory_numa_{}", info.name),
                format!("Memory information field {}.", info.name),
                info.value,
                tags!("node" => numa),
            ));
        }

        let content = std::fs::read_to_string(entry.path().join("numastat"))?;
        for (key, value) in parse_meminfo_numa_stat(&content)? {
            metrics.push(Metric::sum_with_tags(
                format!("node_memory_numa_{}_total", key),
                format!("Memory information field {}", key),
                value,
                tags!("node" => numa),
            ))
        }
    }

    Ok(metrics)
}

struct Meminfo<'a> {
    name: Cow<'a, str>,
    numa: &'a str,
    value: f64,
}

// parsing something like
// Node 2 Active(anon):     590464 kB
// Node 2 AnonHugePages:     90112 kB
// Node 2 HugePages_Total:     0
fn parse_meminfo_numa(content: &str) -> Result<Vec<Meminfo<'_>>, Error> {
    let mut infos = Vec::new();
    for line in content.lines() {
        // first part is always `Node`
        let mut parts = line.split_ascii_whitespace().skip(1);
        let Some(numa) = parts.next() else { continue };

        let name = match parts.next() {
            Some(name) => sanitize(name),
            None => continue,
        };

        let value = match parts.next() {
            Some(value) => value.parse::<f64>()?,
            None => continue,
        };

        let multi = match parts.next() {
            Some(multi) => {
                if multi == "kB" {
                    1024f64
                } else {
                    return Err(Error::Malformed("unit"));
                }
            }
            None => 1.0,
        };

        infos.push(Meminfo {
            name,
            numa,
            value: value * multi,
        });
    }

    Ok(infos)
}

// something like
// local_node 26719046550
// other_node 9860526920
fn parse_meminfo_numa_stat(content: &str) -> Result<Vec<(&str, f64)>, Error> {
    let mut stats = Vec::new();
    for line in content.lines() {
        let mut parts = line.split_ascii_whitespace();
        let Some(name) = parts.next() else { continue };
        let value = match parts.next() {
            Some(value) => value.parse()?,
            None => continue,
        };

        stats.push((name, value));
    }

    Ok(stats)
}

// Active(anon) -> Active_anon
fn sanitize(name: &str) -> Cow<'_, str> {
    let name = name.trim_end_matches([':', ')']);
    if name.contains('(') {
        return Cow::Owned(name.replace('(', "_"));
    }

    Cow::Borrowed(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn smoke() {
        let paths = Paths::test();
        let metrics = collect(paths).await.unwrap();
        assert_ne!(metrics.len(), 0);
    }

    #[test]
    fn sanitize_names() {
        for (input, want) in [
            ("Active(anon)", "Active_anon"),
            ("Inactive(anon)", "Inactive_anon"),
            ("Active(file)", "Active_file"),
            ("Inactive(file)", "Inactive_file"),
        ] {
            let got = sanitize(input);
            assert_eq!(want, got);
        }
    }

    #[test]
    fn meminfo_numa() {
        let content =
            std::fs::read_to_string("tests/node/fixtures/sys/devices/system/node/node0/meminfo")
                .unwrap();
        let infos = parse_meminfo_numa(&content).unwrap();
        assert_eq!(infos[5].value, 707915776.0);
        assert_eq!(infos[5].name, sanitize("Active(anon)"));
        assert_eq!(infos[25].value, 150994944.0);

        let content =
            std::fs::read_to_string("tests/node/fixtures/sys/devices/system/node/node1/meminfo")
                .unwrap();
        let infos = parse_meminfo_numa(&content).unwrap();
        assert_eq!(infos[6].value, 291930112.0);
        assert_eq!(infos[13].value, 85585088512.0);
    }

    #[test]
    fn meminfo_numa_stat() {
        let content =
            std::fs::read_to_string("tests/node/fixtures/sys/devices/system/node/node0/numastat")
                .unwrap();
        let stats = parse_meminfo_numa_stat(&content).unwrap();
        assert_eq!(stats[0], ("numa_hit", 193460335812.0));
        assert_eq!(stats[4].1, 193454780853.0);

        let content =
            std::fs::read_to_string("tests/node/fixtures/sys/devices/system/node/node1/numastat")
                .unwrap();
        let stats = parse_meminfo_numa_stat(&content).unwrap();
        assert_eq!(stats[1].1, 59858626709.0);
        assert_eq!(stats[5].1, 59860526920.0);
    }
}
