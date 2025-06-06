use std::collections::BTreeMap;

use event::{Metric, tags};

use super::Error;
use super::connection::Connection;

pub async fn collect(conn: &mut Connection) -> Result<Vec<Metric>, Error> {
    let infos: Vec<BTreeMap<String, String>> = conn.execute(&["sentinel", "masters"]).await?;

    let mut metrics = Vec::with_capacity(4 * infos.len());
    for info in infos {
        let Some(master) = info.get("name") else {
            continue;
        };

        let Some(ip) = info.get("ip") else {
            continue;
        };

        let Some(port) = info.get("port") else {
            continue;
        };

        let master_addr = format!("{}:{}", ip, port);

        let (status, msg) = match conn
            .execute::<String>(&["sentinel", "ckquorum", master.as_str()])
            .await
        {
            Ok(s) => (1, s),
            Err(err) => (0, err.to_string()),
        };
        metrics.push(Metric::gauge_with_tags(
            "redis_sentinel_master_ckquorum_status",
            "Master ckquorum status",
            status,
            tags!(
                "master" => master,
                "message" => msg,
            ),
        ));

        let quorum = info
            .get("quorum")
            .and_then(|value| value.parse::<f64>().ok())
            .unwrap_or_default();
        let failover_timeout = info
            .get("failover-timeout")
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or_default();
        let parallel_syncs = info
            .get("parallel-syncs")
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or_default();
        let down_after = info
            .get("down-after-milliseconds")
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or_default();

        metrics.extend([
            Metric::gauge_with_tags(
                "redis_sentinel_master_setting_ckquorum",
                "Show the current ckquorum config for each master",
                quorum,
                tags!(
                    "master" => master,
                    "master_address" => &master_addr,
                ),
            ),
            Metric::gauge_with_tags(
                "redis_sentinel_master_setting_failover_timeout",
                "Show the current failover-timeout config for each master",
                failover_timeout,
                tags!(
                    "master" => master,
                    "master_address" => &master_addr,
                ),
            ),
            Metric::gauge_with_tags(
                "redis_sentinel_master_setting_parallel_syncs",
                "Show the current parallel-syncs config for each master",
                parallel_syncs,
                tags!(
                    "master" => master,
                    "master_address" => &master_addr,
                ),
            ),
            Metric::gauge_with_tags(
                "redis_sentinel_master_setting_down_after_milliseconds",
                "Show the current down-after-milliseconds config for each master",
                down_after,
                tags!(
                    "master" => master,
                    "master_address" => &master_addr,
                ),
            ),
        ]);

        if let Ok(partial) = sentinel_info(conn, master, master_addr.as_str()).await {
            metrics.extend(partial);
        }

        if let Ok(partial) = sentinel_slave_info(conn, master, master_addr.as_str()).await {
            metrics.extend(partial);
        }
    }

    Ok(metrics)
}

async fn sentinel_info(
    conn: &mut Connection,
    master: &str,
    master_addr: &str,
) -> Result<Vec<Metric>, Error> {
    let infos: Vec<BTreeMap<String, String>> =
        conn.execute(&["sentinel", "sentinels", master]).await?;

    // If we are here then this master is in ok state
    let mut oks = 1;
    for mut info in infos {
        let Some(flags) = info.remove("flags") else {
            continue;
        };

        if flags.contains("o_down") || flags.contains("s_down") {
            continue;
        }

        oks += 1;
    }

    Ok(vec![Metric::gauge_with_tags(
        "redis_sentinel_master_ok_sentinels",
        "The number of okay sentinels monitoring this master",
        oks,
        tags!(
            "master" => master,
            "master_address" => master_addr
        ),
    )])
}

async fn sentinel_slave_info(
    conn: &mut Connection,
    master: &str,
    master_addr: &str,
) -> Result<Vec<Metric>, Error> {
    let infos: Vec<BTreeMap<String, String>> =
        conn.execute(&["sentinel", "slaves", master]).await?;

    let mut oks = 0;
    for mut info in infos {
        let Some(flags) = info.remove("flags") else {
            continue;
        };

        if flags.contains("o_down") || flags.contains("s_down") {
            continue;
        }

        oks += 1;
    }

    Ok(vec![Metric::gauge_with_tags(
        "redis_sentinel_master_ok_slaves",
        "The number of okay slaves of the master",
        oks,
        tags!(
            "master" => master,
            "master_address" => master_addr
        ),
    )])
}
