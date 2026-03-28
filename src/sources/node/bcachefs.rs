use std::io::ErrorKind;

use event::{Metric, tags};

use super::{Error, Paths, read_file_no_stat};

pub async fn collect(paths: Paths) -> Result<Vec<Metric>, Error> {
    let dirs = std::fs::read_dir(paths.sys().join("fs/bcachefs")).map_err(|err| {
        if err.kind() == ErrorKind::NotFound {
            Error::NoData
        } else {
            err.into()
        }
    })?;

    let mut metrics = Vec::new();
    for entry in dirs.flatten() {
        let Ok(typ) = entry.file_type() else {
            continue;
        };
        if !typ.is_dir() {
            continue;
        }

        let uuid = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();

        let content = read_file_no_stat(path.join("btree_cache_size"))?;
        let btree_cache_size = parse_human_readable_bytes(content.trim())?;
        metrics.extend([
            Metric::gauge_with_tags(
                "node_bcachefs_info",
                "Filesystem information.",
                1,
                tags!( "uuid" => &uuid ),
            ),
            Metric::gauge_with_tags(
                "node_bcachefs_btree_cache_size_bytes",
                "Btree cache memory usage in bytes.",
                btree_cache_size,
                tags!( "uuid" => &uuid ),
            ),
        ]);

        let content = read_file_no_stat(path.join("compression_stats"))?;
        for (algorithm, stats) in parse_compression_stats(&content)? {
            metrics.extend([
                Metric::gauge_with_tags(
                    "node_bcachefs_compression_compressed_bytes",
                    "Compressed size by algorithm.",
                    stats.compressed_bytes,
                    tags!(
                        "algorithm" => algorithm,
                        "uuid" => &uuid,
                    ),
                ),
                Metric::gauge_with_tags(
                    "node_bcachefs_compression_uncompressed_bytes",
                    "Uncompressed size by algorithm.",
                    stats.uncompressed_bytes,
                    tags!(
                        "algorithm" => algorithm,
                        "uuid" => &uuid,
                    ),
                ),
            ]);
        }

        let content = read_file_no_stat(path.join("errors"))?;
        for (typ, stats) in parse_errors(&content)? {
            metrics.push(Metric::sum_with_tags(
                "node_bcachefs_errors_total",
                "Error count by error type.",
                stats.count,
                tags!(
                    "error_type" => typ,
                    "uuid" => &uuid
                ),
            ))
        }

        let content = read_file_no_stat(path.join("btree_write_stats"))?;
        for (typ, stats) in parse_btree_write_stats(&content)? {
            metrics.extend([
                Metric::sum_with_tags(
                    "node_bcachefs_btree_writes_total",
                    "Number of btree writes by type.",
                    stats.count,
                    tags!(
                        "type" => typ,
                        "uuid" => &uuid
                    ),
                ),
                Metric::gauge_with_tags(
                    "node_bcachefs_btree_write_average_size_bytes",
                    "Average btree write size by type.",
                    stats.size_bytes,
                    tags!(
                        "type" => typ,
                        "uuid" => &uuid
                    ),
                ),
            ]);
        }

        let dirs = std::fs::read_dir(path.join("counters"))?;
        for entry in dirs.flatten() {
            let Ok(typ) = entry.file_type() else { continue };
            if !typ.is_file() {
                continue;
            }

            let content = read_file_no_stat(entry.path())?;
            let Ok(counter) = parse_counter(&content) else {
                continue;
            };

            let name = entry.file_name().to_string_lossy().into_owned();
            metrics.push(Metric::sum_with_tags(
                format!(
                    "node_bcachefs_{}_total",
                    sanitize_metric_name(name.as_ref())
                ),
                format!("Bcachefs counter {name} since filesystem creation."),
                counter.since_filesystem_creation,
                tags!("uuid" => &uuid),
            ));
        }

        let dirs = std::fs::read_dir(path)?;
        for entry in dirs.flatten() {
            let Ok(typ) = entry.file_type() else { continue };
            if !typ.is_dir() {
                continue;
            }

            let name = entry.file_name().to_string_lossy().into_owned();
            let Some(device) = name.strip_prefix("dev-") else {
                continue;
            };

            let path = entry.path();
            let label = read_file_no_stat(path.join("label")).unwrap_or_default();
            let state = read_file_no_stat(path.join("state")).unwrap_or_default();
            let bucket_size = match read_file_no_stat(path.join("bucket_size")) {
                Ok(content) => match parse_human_readable_bytes(content.trim()) {
                    Ok(size) => size,
                    Err(_) => continue,
                },
                Err(_) => continue,
            };

            let buckets = match read_file_no_stat(path.join("nbuckets")) {
                Ok(content) => match content.trim().parse::<usize>() {
                    Ok(buckets) => buckets,
                    Err(_) => continue,
                },
                Err(_) => continue,
            };
            let durability = match read_file_no_stat(path.join("durability")) {
                Ok(content) => match content.trim().parse::<usize>() {
                    Ok(durability) => durability,
                    Err(_) => continue,
                },
                Err(_) => continue,
            };
            metrics.extend([
                Metric::gauge_with_tags(
                    "node_bcachefs_device_info",
                    "Device information.",
                    1,
                    tags!(
                        "device" => device,
                        "label" => label.trim(),
                        "state" => state.trim(),
                        "uuid" => &uuid
                    ),
                ),
                Metric::gauge_with_tags(
                    "node_bcachefs_device_bucket_size_bytes",
                    "Bucket size in bytes",
                    bucket_size,
                    tags!(
                        "device" => device,
                        "uuid" => &uuid
                    ),
                ),
                Metric::gauge_with_tags(
                    "node_bcachefs_device_buckets",
                    "Total number of buckets.",
                    buckets,
                    tags!(
                        "device" => device,
                        "uuid" => &uuid
                    ),
                ),
                Metric::gauge_with_tags(
                    "node_bcachefs_device_durability",
                    "Device durability setting.",
                    durability,
                    tags!(
                        "device" => device,
                        "uuid" => &uuid
                    ),
                ),
            ]);

            let Ok(content) = read_file_no_stat(path.join("io_done")) else {
                continue;
            };
            for (operation, stats) in parse_device_io_done(&content)? {
                for (typ, value) in stats {
                    metrics.push(Metric::sum_with_tags(
                        "node_bcachefs_device_io_done_bytes_total",
                        "IO bytes by operation type and data type.",
                        value,
                        tags!(
                            "device" => device,
                            "data_type" => typ,
                            "operation" => operation,
                            "uuid" => &uuid
                        ),
                    ));
                }
            }

            let Ok(content) = read_file_no_stat(path.join("io_errors")) else {
                continue;
            };
            for (typ, value) in parse_device_io_errors(&content)? {
                metrics.push(Metric::sum_with_tags(
                    "node_bcachefs_device_io_errors_total",
                    "IO errors by error type.",
                    value,
                    tags!(
                        "device" => device,
                        "type" => typ,
                        "uuid" => &uuid
                    ),
                ))
            }
        }
    }

    Ok(metrics)
}

fn sanitize_metric_name(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut last: char = char::MIN;
    for c in input.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
            last = char::MIN;
            continue;
        }

        if last == '_' {
            continue;
        }

        last = '_';
        out.push('_');
    }

    out
}

/// Compression statistics for a specific algorithm
struct CompressionStats {
    compressed_bytes: usize,
    uncompressed_bytes: usize,
    // average_extent_size_bytes: usize,
}

fn parse_compression_stats(content: &str) -> Result<Vec<(&str, CompressionStats)>, Error> {
    let mut stats = Vec::new();
    for line in content.lines().skip(1) {
        let mut fields = line.split_ascii_whitespace();
        let Some(name) = fields.next() else {
            continue;
        };

        let compressed_bytes = match fields.next() {
            Some(field) => parse_human_readable_bytes(field)?,
            None => continue,
        };
        let uncompressed_bytes = match fields.next() {
            Some(field) => parse_human_readable_bytes(field)?,
            None => continue,
        };
        // let average_extent_size_bytes = match fields.next() {
        //     Some(field) => parse_human_readable_bytes(field)?,
        //     None => 0,
        // };

        stats.push((
            name,
            CompressionStats {
                compressed_bytes,
                uncompressed_bytes,
                // average_extent_size_bytes,
            },
        ))
    }

    Ok(stats)
}

/// Error count and timestamp for a specific error type
struct ErrorStats {
    count: u64,
    // timestamp: u64,
}

fn parse_errors(content: &str) -> Result<Vec<(&str, ErrorStats)>, Error> {
    let mut stats = Vec::new();
    for line in content.lines() {
        let mut fields = line.split_ascii_whitespace();

        let Some(typ) = fields.next() else {
            continue;
        };
        let count = match fields.next() {
            Some(field) => field.parse::<u64>()?,
            None => continue,
        };
        // let timestamp = match fields.next() {
        //     Some(field) => field.parse::<u64>()?,
        //     None => 0,
        // };

        stats.push((
            typ,
            ErrorStats {
                count, /* timestamp */
            },
        ))
    }

    Ok(stats)
}

/// Counter values since mount and since filesystem creation
#[derive(Default)]
struct CounterStats {
    since_mount: usize,
    since_filesystem_creation: usize,
}

fn parse_counter(content: &str) -> Result<CounterStats, Error> {
    let mut stats = CounterStats::default();

    for line in content.lines() {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };

        match name {
            "since mount" => {
                stats.since_mount = parse_human_readable_bytes(value.trim())?;
            }
            "since filesystem creation" => {
                stats.since_filesystem_creation = parse_human_readable_bytes(value.trim())?;
            }
            _ => {}
        }
    }

    Ok(stats)
}

/// Btree write statistics for a specific type
struct BtreeWriteStats {
    count: u64,
    size_bytes: usize,
}

fn parse_btree_write_stats(content: &str) -> Result<Vec<(&str, BtreeWriteStats)>, Error> {
    let mut stats = Vec::new();

    for line in content.lines().skip(1) {
        let mut fields = line.split_ascii_whitespace();
        let Some(typ) = fields.next() else {
            continue;
        };

        let count = match fields.next() {
            Some(field) => field.parse::<u64>()?,
            None => continue,
        };
        let size_bytes = match fields.next() {
            Some(field) => parse_human_readable_bytes(field)?,
            None => continue,
        };

        stats.push((
            typ.strip_suffix(':').unwrap_or(typ),
            BtreeWriteStats { count, size_bytes },
        ));
    }

    Ok(stats)
}

fn parse_device_io_done(content: &str) -> Result<Vec<(&str, Vec<(&str, usize)>)>, Error> {
    let mut stats = Vec::with_capacity(2);

    let mut operation = "";
    let mut stat = Vec::with_capacity(10);
    for line in content.lines() {
        if line.starts_with("read:") {
            if !stat.is_empty() {
                stats.push((operation, stat));
                stat = Vec::new();
            }

            operation = "read";
            continue;
        } else if line.starts_with("write:") {
            if !stat.is_empty() {
                stats.push((operation, stat));
                stat = Vec::new();
            }

            operation = "write";
            continue;
        }

        let Some((typ, value)) = line.split_once(':') else {
            continue;
        };

        let value = value.trim().parse::<usize>()?;

        stat.push((typ.trim(), value));
    }

    stats.push((operation, stat));

    Ok(stats)
}

fn parse_device_io_errors(content: &str) -> Result<Vec<(&str, usize)>, Error> {
    let mut stats = Vec::with_capacity(4);

    let mut creation_section = false;
    for line in content.lines() {
        if !creation_section && line.starts_with("IO errors since filesystem creation") {
            creation_section = true;
            continue;
        }

        if line.starts_with("IO errors since ") {
            break;
        }

        let Some((first, value)) = line.split_once(':') else {
            continue;
        };

        let value = value.trim().parse::<usize>()?;
        stats.push((first.trim(), value));
    }

    Ok(stats)
}

fn parse_human_readable_bytes(input: &str) -> Result<usize, Error> {
    let len = input.len();

    let (first, second) = input.split_at(len - 1);
    let (value, multi) = match second {
        "k" | "K" => (first, 1024u64),
        "m" | "M" => (first, 1024 * 1024),
        "g" | "G" => (first, 1024 * 1024 * 1024),
        "t" | "T" => (first, 1024 * 1024 * 1024 * 1024),
        "p" | "P" => (first, 1024 * 1024 * 1024 * 1024 * 1024),
        "e" | "E" => (first, 1024 * 1024 * 1024 * 1024 * 1024 * 1024),
        _ => (input, 1),
    };

    let value = value.parse::<f64>()?;

    Ok((value * multi as f64) as usize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn smoke() {
        let paths = Paths::test();
        let metrics = collect(paths).await.unwrap();
        assert_ne!(metrics.len(), 0);
    }

    #[test]
    fn human_readable_bytes() {
        for (input, want) in [
            ("542k", 555008),
            ("322M", 337641472),
            ("1.5G", 1610612736),
            ("112k", 114688),
            ("405", 405),
        ] {
            let got = parse_human_readable_bytes(input.trim()).unwrap();
            assert_eq!(want, got);
        }
    }

    #[test]
    fn sanitize() {
        for (input, want) in [
            ("", ""),
            ("rx_errors", "rx_errors"),
            ("Queue[0] AllocFails", "Queue_0_AllocFails"),
            ("Tx LPI entry count", "Tx_LPI_entry_count"),
            (
                "port.VF_admin_queue_requests",
                "port_VF_admin_queue_requests",
            ),
            ("[3]: tx_bytes", "_3_tx_bytes"),
            ("     err", "_err"),
        ] {
            let got = sanitize_metric_name(input);
            assert_eq!(want, got);
        }
    }

    #[test]
    fn device_io_errors() {
        let content = r#"IO errors since filesystem creation
  read:    197416
  write:   205
  checksum:0
IO errors since 8 y ago
  read:    197416
  write:   205
  checksum:0
"#;
        let got = parse_device_io_errors(content).unwrap();
        assert_eq!(vec![("read", 197416), ("write", 205), ("checksum", 0)], got);
    }

    #[test]
    fn device_io_done() {
        let content = r#"read:
sb          :       86016
journal     :           0
btree       :  2193358848
user        :3770452246528
cached      :           0
parity      :           0
stripe      :           0
need_gc_gens:           0
need_discard:           0
unstriped   :           0
write:
sb          :      645120
journal     :           0
btree       :      589824
user        :6258285805568
cached      :           0
parity      :           0
stripe      :           0
need_gc_gens:           0
need_discard:           0
unstriped   :           0
"#;
        let got = parse_device_io_done(content).unwrap();
        assert_eq!(
            vec![
                (
                    "read",
                    vec![
                        ("sb", 86016),
                        ("journal", 0),
                        ("btree", 2193358848),
                        ("user", 3770452246528),
                        ("cached", 0),
                        ("parity", 0),
                        ("stripe", 0),
                        ("need_gc_gens", 0),
                        ("need_discard", 0),
                        ("unstriped", 0),
                    ]
                ),
                (
                    "write",
                    vec![
                        ("sb", 645120),
                        ("journal", 0),
                        ("btree", 589824),
                        ("user", 6258285805568),
                        ("cached", 0),
                        ("parity", 0),
                        ("stripe", 0),
                        ("need_gc_gens", 0),
                        ("need_discard", 0),
                        ("unstriped", 0),
                    ]
                )
            ],
            got
        );
    }

    #[test]
    fn counter_stats() {
        let root = PathBuf::from(
            "tests/node/fixtures/sys/fs/bcachefs/deadbeef-1234-5678-9012-abcdefabcdef/counters",
        );
        let content = read_file_no_stat(root.join("btree_node_read")).unwrap();
        let stats = parse_counter(&content).unwrap();
        assert_eq!(stats.since_filesystem_creation, 70225874);

        let content = read_file_no_stat(root.join("reconcile_btree")).unwrap();
        let stats = parse_counter(&content).unwrap();
        assert_eq!(
            stats.since_filesystem_creation,
            (2.01 * 1024.0 * 1024.0 * 1024.0) as usize
        ); // 2.01G
    }
}
