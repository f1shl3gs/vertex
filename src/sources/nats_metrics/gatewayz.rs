use std::collections::BTreeMap;
use std::time::Duration;

use chrono::{DateTime, Utc};
use event::{Metric, tags};
use framework::http::HttpClient;
use serde::Deserialize;

use super::{Error, fetch};

#[derive(Deserialize)]
struct Gateway {
    name: String,
    outbound_gateways: BTreeMap<String, RemoteGateway>,
    inbound_gateways: BTreeMap<String, Vec<RemoteGateway>>,
}

#[derive(Deserialize)]
struct RemoteGateway {
    configured: bool,
    connection: Connection,
}

impl RemoteGateway {
    fn metrics(&self, typ: &str, gateway: &str, remote: &str) -> Vec<Metric> {
        let tags = tags!(
            "cid" => self.connection.cid,
            "gateway" => gateway,
            "remote_gateway" => remote,
        );

        let mut metrics = vec![
            Metric::gauge_with_tags(
                format!("gnatsd_gatewayz_{}_configured", typ),
                "",
                self.configured,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                format!("gnatsd_gatewayz_{}_conn_start_time_seconds", typ),
                "",
                self.connection.start.timestamp(),
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                format!("gnatsd_gatewayz_{}_conn_last_active_seconds", typ),
                "",
                self.connection.last_activity.timestamp(),
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                format!("gnatsd_gatewayz_{}_conn_uptime_seconds", typ),
                "",
                self.connection.uptime,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                format!("gnatsd_gatewayz_{}_conn_idle_seconds", typ),
                "",
                self.connection.idle,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                format!("gnatsd_gatewayz_{}_conn_pending_bytes", typ),
                "",
                self.connection.pending_bytes,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                format!("gnatsd_gatewayz_{}_conn_in_msgs", typ),
                "",
                self.connection.in_msgs,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                format!("gnatsd_gatewayz_{}_conn_out_msgs", typ),
                "",
                self.connection.out_msgs,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                format!("gnatsd_gatewayz_{}_conn_in_bytes", typ),
                "",
                self.connection.in_bytes,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                format!("gnatsd_gatewayz_{}_conn_out_bytes", typ),
                "",
                self.connection.out_bytes,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                format!("gnatsd_gatewayz_{}_conn_subscriptions", typ),
                "",
                self.connection.subscriptions,
                tags.clone(),
            ),
        ];

        if let Some(rtt) = self.connection.rtt {
            metrics.push(Metric::gauge_with_tags(
                format!("gnatsd_gatewayz_{}_conn_rtt", typ),
                "",
                rtt,
                tags,
            ));
        }

        metrics
    }
}

#[derive(Deserialize)]
struct Connection {
    cid: u64,

    start: DateTime<Utc>,
    last_activity: DateTime<Utc>,
    #[serde(default, with = "humanize::duration::serde_option")]
    rtt: Option<Duration>,
    #[serde(with = "humanize::duration::serde")]
    uptime: Duration,
    #[serde(with = "humanize::duration::serde")]
    idle: Duration,
    pending_bytes: i64,
    in_msgs: i64,
    out_msgs: i64,
    in_bytes: i64,
    out_bytes: i64,
    subscriptions: i64,
}

pub async fn collect(client: &HttpClient, endpoint: &str) -> Result<Vec<Metric>, Error> {
    let resp = fetch::<Gateway>(client, &format!("{endpoint}/gatewayz")).await?;

    let mut metrics = Vec::new();
    for (name, inbounds) in resp.inbound_gateways {
        for inbound in inbounds {
            metrics.extend(inbound.metrics("inbound", &resp.name, &name));
        }
    }

    for (name, outbound) in resp.outbound_gateways {
        metrics.extend(outbound.metrics("outbound", &resp.name, &name));
    }

    Ok(metrics)
}
