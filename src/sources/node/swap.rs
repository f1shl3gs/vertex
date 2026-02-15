use std::path::PathBuf;

use event::{Metric, tags};

use super::Error;

// Swap represents an entry in /proc/swaps
#[cfg_attr(test, derive(Debug, PartialEq))]
struct Swap<'a> {
    device: &'a str,
    typ: &'a str,
    size: usize,
    used: usize,
    priority: i32,
}

pub async fn gather(proc: PathBuf) -> Result<Vec<Metric>, Error> {
    let content = std::fs::read_to_string(proc.join("swaps"))?;
    let mut metrics = Vec::new();

    // skip header line
    for line in content.lines().skip(1) {
        let swap = parse_swap_line(line)?;
        let tags = tags!(
            "device" => swap.device,
            "swap_type" => swap.typ,
        );

        metrics.extend([
            Metric::gauge_with_tags(
                "node_swap_size_bytes",
                "Swap device size in bytes",
                swap.size * 1024,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "node_swap_used_bytes",
                "Swap device used in bytes",
                swap.used * 1024,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "node_swap_priority",
                "Swap device priority",
                swap.priority,
                tags,
            ),
        ]);
    }

    if metrics.is_empty() {
        return Err(Error::NoData);
    }

    Ok(metrics)
}

fn parse_swap_line(line: &str) -> Result<Swap<'_>, Error> {
    let mut parts = line.split_ascii_whitespace();

    let Some(filename) = parts.next() else {
        return Err(Error::Malformed("filename"));
    };

    let Some(typ) = parts.next() else {
        return Err(Error::Malformed("type"));
    };

    let size = match parts.next() {
        Some(size) => size.parse()?,
        None => return Err(Error::Malformed("size")),
    };

    let used = match parts.next() {
        Some(used) => used.parse()?,
        None => return Err(Error::Malformed("used")),
    };

    let priority = match parts.next() {
        Some(priority) => priority.parse()?,
        None => return Err(Error::Malformed("priority")),
    };

    Ok(Swap {
        device: filename,
        typ,
        size,
        used,
        priority,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        for (line, expect) in [
            (
                "/dev/dm-2                               partition       131068  1024    -2",
                Some(Swap {
                    device: "/dev/dm-2",
                    typ: "partition",
                    size: 131068,
                    used: 1024,
                    priority: -2,
                }),
            ),
            (
                "/foo                                    file            1048572 0       -3",
                Some(Swap {
                    device: "/foo",
                    typ: "file",
                    size: 1048572,
                    used: 0,
                    priority: -3,
                }),
            ),
            (
                "/dev/sda2                               partition       hello   world   -2",
                None,
            ),
            (
                "/dev/dm-2                               partition       131068  1024",
                None,
            ),
        ] {
            let got = parse_swap_line(line).ok();
            assert_eq!(got, expect, "{line}");
        }
    }
}
