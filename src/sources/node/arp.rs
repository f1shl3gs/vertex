//! Exposes ARP statistics from `/proc/net/arp`.
use std::collections::HashMap;

use event::{tags, Metric};
use tokio::io::AsyncBufReadExt;

use super::Error;

pub async fn gather(proc_path: &str) -> Result<Vec<Metric>, Error> {
    let path = format!("{}/net/arp", proc_path);
    let f = tokio::fs::File::open(&path)
        .await
        .map_err(|err| Error::Io {
            err,
            msg: "open arp file failed".into(),
        })?;
    let reader = tokio::io::BufReader::new(f);
    let mut lines = reader.lines();
    let mut devices = HashMap::<String, i64>::new();

    // skip the first line
    lines.next_line().await?;

    while let Some(line) = lines.next_line().await? {
        let dev = line.split_ascii_whitespace().nth(5).unwrap();

        match devices.get_mut(dev) {
            Some(v) => *v += 1i64,
            _ => {
                devices.insert(dev.into(), 1i64);
            }
        }
    }

    let mut metrics = Vec::with_capacity(devices.len());
    for (device, count) in devices {
        metrics.push(Metric::gauge_with_tags(
            "node_arp_entries",
            "",
            count as f64,
            tags!(
                "device" => &device,
            ),
        ));
    }

    Ok(metrics)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gather() {
        let proc_path = "tests/fixtures/proc";
        gather(proc_path).await.unwrap();
    }
}
