//! Exposes ARP statistics from `/proc/net/arp`.

use std::collections::HashMap;
use std::path::PathBuf;

use event::{Metric, tags};

use super::Error;

pub async fn gather(proc_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let data = std::fs::read_to_string(proc_path.join("net/arp"))?;

    let mut devices = HashMap::new();
    // the first line is title, so we don't need it
    for line in data.lines().skip(1) {
        let dev = line.split_ascii_whitespace().nth(5).expect("must exists");

        devices
            .entry(dev)
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
