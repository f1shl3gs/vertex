/// Exposes ARP statistics from `/proc/net/arp`.

use crate::{
    event::{Metric, MetricValue},
    tags,
};
use std::path::PathBuf;
use tokio::io::{
    AsyncBufReadExt
};
use std::collections::{
    HashMap
};
use crate::sources::node::errors::Error;

pub async fn gather(proc_path: &str) -> Result<Vec<Metric>, Error> {
    let mut path = PathBuf::from(proc_path);
    path.push("net/arp");

    let f = tokio::fs::File::open(path).await?;
    let reader = tokio::io::BufReader::new(f);
    let mut lines = reader.lines();
    let mut devices = HashMap::<String, i64>::new();

    // skip the first line
    lines.next_line().await?;

    while let Some(line) = lines.next_line().await? {
        let dev = line.split_ascii_whitespace()
            .nth(5)
            .unwrap();

        match devices.get_mut(dev) {
            Some(v) => *v += 1i64,
            _ => {
                devices.insert(dev.into(), 1i64);
            }
        }
    }

    let mut metrics = Vec::with_capacity(devices.len());
    for (device, count) in devices {
        metrics.push(Metric {
            name: "node_arp_entries".into(),
            description: None,
            tags: tags!(
                "device" => device,
            ),
            unit: None,
            timestamp: 0,
            value: MetricValue::Gauge(count as f64),
        })
    }

    Ok(metrics)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gather() {
        let proc_path = "testdata/proc";
        gather(proc_path).await.unwrap();
    }
}