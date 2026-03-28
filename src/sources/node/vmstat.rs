//! Exposes statistics from `/proc/vmstat`

use configurable::Configurable;
use event::Metric;
use framework::config::serde_regex;
use regex::Regex;
use serde::{Deserialize, Serialize};

use super::{Error, Paths, read_file_no_stat};

fn default_fields() -> Regex {
    Regex::new("^(oom_kill|pgpg|pswp|pg.*fault).*").unwrap()
}

#[derive(Clone, Configurable, Debug, Deserialize, Serialize)]
pub struct Config {
    /// Regexp of fields to return for vmstat collector.
    #[serde(default = "default_fields", with = "serde_regex")]
    fields: Regex,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            fields: default_fields(),
        }
    }
}

pub async fn collect(conf: Config, paths: Paths) -> Result<Vec<Metric>, Error> {
    let content = read_file_no_stat(paths.proc().join("vmstat"))?;

    let mut metrics = Vec::new();
    for line in content.lines() {
        if !conf.fields.is_match(line) {
            continue;
        }

        let mut fields = line.split_ascii_whitespace();
        let Some(key) = fields.next() else {
            continue;
        };
        let Some(value) = fields.next() else {
            continue;
        };

        if let Ok(value) = value.parse::<f64>() {
            metrics.push(Metric::gauge(
                format!("node_vmstat_{key}"),
                format!("/proc/vmstat information field {key}"),
                value,
            ))
        }
    }

    Ok(metrics)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn smoke() {
        let conf = Config::default();
        let paths = Paths::test();
        let metrics = collect(conf, paths).await.unwrap();
        assert_ne!(metrics.len(), 0);
    }
}
