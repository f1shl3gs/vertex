mod hosts;
mod status;

use configurable::Configurable;
use event::Metric;
use framework::config::default_true;
use serde::{Deserialize, Serialize};

use super::connection::{Connection, Error, Flavor};

#[derive(Configurable, Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    /// Since 5.1, Collect from SHOW SLAVE STATUS (Enabled by default)
    #[serde(default = "default_true")]
    status: bool,

    /// Since 5.1, Scrape information from 'SHOW SLAVE HOSTS'
    #[serde(default)]
    hosts: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            status: true,
            hosts: false,
        }
    }
}

pub async fn collect(conn: &mut Connection, conf: &Config) -> Result<Vec<Metric>, Error> {
    let version = conn.version();
    let mut metrics = Vec::new();

    if conf.status && version >= 5.1 {
        metrics.extend(status::collect(conn).await?);
    }

    if conf.hosts && version >= 5.1 {
        metrics.extend(hosts::collect(conn).await?);
    }

    Ok(metrics)
}
