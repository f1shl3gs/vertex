//! Exposes ARP statistics from `/proc/net/arp`.

use std::collections::HashMap;

use event::{Metric, tags};

use super::{Error, Paths, read_file_no_stat};

pub async fn collect(paths: Paths) -> Result<Vec<Metric>, Error> {
    let content = read_file_no_stat(paths.proc().join("net/arp"))?;

    let mut devices = HashMap::new();
    // the first line is title, so we don't need it
    for line in content.lines().skip(1) {
        let Some(device) = line.split_ascii_whitespace().nth(5) else {
            continue;
        };

        devices
            .entry(device)
            .and_modify(|count| *count += 1)
            .or_insert(1);
    }

    let mut metrics = Vec::with_capacity(devices.len());
    for (device, count) in devices {
        metrics.push(Metric::gauge_with_tags(
            "node_arp_entries",
            "ARP entries by device",
            count,
            tags!("device" => device),
        ));
    }

    Ok(metrics)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn smoke() {
        let paths = Paths::test();
        let metrics = collect(paths).await.unwrap();
        assert_eq!(metrics.len(), 3);
    }
}
