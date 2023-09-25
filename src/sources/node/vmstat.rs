//! Exposes statistics from `/proc/vmstat`
use event::Metric;
use framework::config::serde_regex;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncBufReadExt;

use super::{Error, ErrorContext};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VMStatConfig {
    #[serde(default = "default_fields")]
    #[serde(with = "serde_regex")]
    pub fields: regex::Regex,
}

impl Default for VMStatConfig {
    fn default() -> Self {
        Self {
            fields: default_fields(),
        }
    }
}

fn default_fields() -> regex::Regex {
    const DEFAULT_PATTERN: &str = "^(oom_kill|pgpg|pswp|pg.*fault).*";
    regex::Regex::new(DEFAULT_PATTERN).unwrap()
}

impl VMStatConfig {
    pub async fn gather(&self, proc_path: &str) -> Result<Vec<Metric>, Error> {
        let path = format!("{}/vmstat", proc_path);
        let f = tokio::fs::File::open(path)
            .await
            .context("open vmstat failed")?;

        let r = tokio::io::BufReader::new(f);
        let mut lines = r.lines();
        let mut metrics = Vec::new();

        while let Some(line) = lines.next_line().await.context("read next line failed")? {
            if !self.fields.is_match(&line) {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gather() {
        let conf = VMStatConfig::default();
        let proc = "tests/fixtures/proc";

        let ms = conf.gather(proc).await.unwrap();
        assert_ne!(ms.len(), 0);
    }
}
