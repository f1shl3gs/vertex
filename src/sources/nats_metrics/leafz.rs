use std::time::Duration;

use event::{Metric, tags};
use framework::http::HttpClient;
use serde::Deserialize;

use super::{Error, fetch};

#[derive(Deserialize)]
struct Leafz {
    #[serde(rename = "leafnodes")]
    leaf_nodes: i32,
    leafs: Vec<Leaf>,
}

#[derive(Deserialize)]
struct Leaf {
    #[serde(default)]
    name: String,
    account: String,
    ip: String,
    port: u16,
    #[serde(with = "humanize::duration::serde")]
    rtt: Duration,
    in_msgs: i64,
    out_msgs: i64,
    in_bytes: i64,
    out_bytes: i64,
    subscriptions: i64,
    subscriptions_list: Vec<String>,
}

impl Leaf {
    fn generate_metrics(self) -> Vec<Metric> {
        let mut metrics = self
            .subscriptions_list
            .into_iter()
            .map(|sub| {
                Metric::gauge_with_tags(
                    "gnatds_leafz_conn_subscriptions",
                    "",
                    0,
                    tags!(
                        "account" => self.account.clone(),
                        "account_id" => self.account.clone(),
                        "ip" => self.ip.clone(),
                        "port" => self.port,
                        "name" => self.name.clone(),
                        "subscription" => sub,
                    ),
                )
            })
            .collect::<Vec<_>>();

        let tags = tags!(
            "account" => self.account.clone(),
            "account_id" => self.account,
            "ip" => self.ip,
            "port" => self.port,
            "name" => self.name
        );
        metrics.extend([
            Metric::gauge_with_tags("gnatds_leafz_info", "", 1, tags.clone()),
            Metric::gauge_with_tags("gnatds_leafz_conn_rtt", "", self.rtt, tags.clone()),
            Metric::gauge_with_tags("gnatds_leafz_conn_in_msgs", "", self.in_msgs, tags.clone()),
            Metric::gauge_with_tags(
                "gnatds_leafz_conn_out_msgs",
                "",
                self.out_msgs,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "gnatds_leafz_conn_in_bytes",
                "",
                self.in_bytes,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "gnatds_leafz_conn_out_bytes",
                "",
                self.out_bytes,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "gnatds_leafz_conn_subscriptions_total",
                "",
                self.subscriptions,
                tags,
            ),
        ]);

        metrics
    }
}

pub async fn collect(client: &HttpClient, endpoint: &str) -> Result<Vec<Metric>, Error> {
    let resp = fetch::<Leafz>(client, &format!("{endpoint}/leafz")).await?;

    let mut metrics = Vec::with_capacity(1 + 7 * resp.leafs.len());

    metrics.push(Metric::gauge(
        "gnatds_leafz_conn_nodes_total",
        "",
        resp.leaf_nodes,
    ));

    for leaf in resp.leafs {
        metrics.extend(leaf.generate_metrics());
    }

    Ok(metrics)
}
