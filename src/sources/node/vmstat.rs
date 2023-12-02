//! Exposes statistics from `/proc/vmstat`

use std::path::PathBuf;

use event::Metric;
use framework::config::serde_regex;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncBufReadExt;

use super::Error;

fn default_fields() -> Regex {
    const DEFAULT_PATTERN: &str = "^(oom_kill|pgpg|pswp|pg.*fault).*";
    Regex::new(DEFAULT_PATTERN).unwrap()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    #[serde(default = "default_fields")]
    #[serde(with = "serde_regex")]
    pub fields: Regex,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            fields: default_fields(),
        }
    }
}

pub async fn gather(conf: Config, proc_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let file = tokio::fs::File::open(proc_path.join("vmstat")).await?;
    let mut lines = tokio::io::BufReader::new(file).lines();
    let mut metrics = Vec::new();

    while let Some(line) = lines.next_line().await? {
        if !conf.fields.is_match(&line) {
            continue;
        }

        let parts = line.split_ascii_whitespace().collect::<Vec<_>>();
        if parts.len() != 2 {
            continue;
        }

        match parts[1].parse::<f64>() {
            Ok(v) => metrics.push(Metric::gauge(
                format!("node_vmstat_{}", parts[0]),
                format!("/proc/vmstat information field {}", parts[0]),
                v,
            )),
            _ => continue,
        }
    }

    Ok(metrics)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gather() {
        let conf = Config::default();
        let proc = "tests/fixtures/proc".into();
        let ms = gather(conf, proc).await.unwrap();
        assert_ne!(ms.len(), 0);
    }
}
