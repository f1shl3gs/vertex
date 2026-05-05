mod status;
mod variables;

use chrono::NaiveDateTime;
use configurable::Configurable;
use event::Metric;
use framework::config::default_true;
use serde::{Deserialize, Serialize};

use super::connection::{Connection, Error};
use super::sanitize;

#[derive(Configurable, Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    /// Since 5.1, Collect from SHOW GLOBAL STATUS (Enabled by default)
    #[serde(default = "default_true")]
    status: bool,

    /// Since 5.1, Collect from SHOW GLOBAL VARIABLES (Enabled by default)
    #[serde(default = "default_true")]
    variables: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            status: true,
            variables: true,
        }
    }
}

pub async fn collect(conn: &mut Connection, conf: &Config) -> Result<Vec<Metric>, Error> {
    let version = conn.version();

    let mut metrics = Vec::new();
    if conf.status && version >= 5.1 {
        match status::collect(conn).await {
            Ok(partial) => metrics.extend(partial),
            Err(err) => {
                warn!(message = "collecting global status failed", %err);
                return Err(err);
            }
        }
    }

    if conf.variables && version >= 5.1 {
        match variables::collect(conn).await {
            Ok(partial) => metrics.extend(partial),
            Err(err) => {
                warn!(message = "collecting global variables failed", %err);
                return Err(err);
            }
        }
    }

    Ok(metrics)
}

fn parse_value(value: &str) -> Option<f64> {
    if ["ON", "Primary", "YES"].contains(&value) {
        return Some(1.0);
    } else if ["non-Primary", "Disconnected", "OFF", "NO", "DISABLED"].contains(&value) {
        // SHOW GLOBAL STATUS like 'wsrep_cluster_status' can return "Primary" or "non-Primary"/"Disconnected"
        return Some(0.0);
    }

    if let Ok(value) = value.parse::<f64>() {
        return Some(value);
    }

    // Apr 29 17:31:45 2036 GMT
    for fmt in ["%Y-%m-%d %H:%M:%S", "%b %d %H:%M:%S %Y %Z"] {
        if let Ok(ts) = NaiveDateTime::parse_from_str(value, fmt)
            .map(|datetime| datetime.and_utc().timestamp_micros() as f64 / 1e6)
        {
            return Some(ts);
        }
    }

    None
}
