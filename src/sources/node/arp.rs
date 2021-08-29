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
    HashMap, BTreeMap
};
use std::sync::Arc;

pub async fn gather(proc_path: Arc<String>) -> Result<Vec<Metric>, ()> {
    let mut path = PathBuf::from(proc_path.as_ref());
    path.push("net/arp");

    let f = tokio::fs::File::open(path).await.unwrap();
    let reader = tokio::io::BufReader::new(f);
    let mut lines = reader.lines();
    let mut devices = HashMap::<String, i64>::new();

    // skip the first line
    lines.next_line().await.unwrap();

    while let Some(line) = lines.next_line().await.unwrap() {
        let dev = line.split_ascii_whitespace().nth(5).unwrap();

        match devices.get_mut(dev) {
            Some(v) => *v = *v + 1i64,
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
        let proc_path = Arc::new("/proc/net/arp".to_string());
        gather(proc_path).await.unwrap();
    }
}