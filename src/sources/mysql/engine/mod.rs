mod innodb;
mod tokudb;

use configurable::Configurable;
use event::Metric;
use serde::{Deserialize, Serialize};

use super::connection::{Connection, Error};

#[derive(Configurable, Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    /// Since 5.1, Collect from SHOW ENGINE INNODB STATUS
    #[serde(default)]
    innodb: bool,

    /// Since 5.6, Collect from SHOW ENGINE TOKUDB STATUS
    #[serde(default)]
    tokudb: bool,
}

pub async fn collect(conn: &mut Connection, conf: &Config) -> Result<Vec<Metric>, Error> {
    let version = conn.version();

    let mut metrics = if conf.innodb && version >= 5.1 {
        innodb::collect(conn).await?
    } else {
        vec![]
    };

    if conf.tokudb && version >= 5.6 {
        metrics.extend(tokudb::collect(conn).await?);
    }

    Ok(metrics)
}
