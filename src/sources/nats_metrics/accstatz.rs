use event::{Metric, tags};
use framework::http::HttpClient;
use serde::Deserialize;

use super::{Error, fetch};

#[derive(Deserialize)]
struct Stats {
    msgs: u64,
    bytes: u64,
}

#[derive(Deserialize)]
struct Account {
    acc: String,
    #[serde(default)]
    name: String,
    conns: i32,
    #[serde(rename = "leafnodes")]
    leaf_nodes: i32,
    total_conns: i32,
    num_subscriptions: u32,
    sent: Stats,
    received: Stats,
    slow_consumers: i64,
}

#[derive(Deserialize)]
struct Accstatz {
    account_statz: Vec<Account>,
}

pub async fn collect(client: &HttpClient, endpoint: &str) -> Result<Vec<Metric>, Error> {
    let resp = fetch::<Accstatz>(client, &format!("{endpoint}/accstatz?unused=1")).await?;

    let mut metrics = Vec::with_capacity(resp.account_statz.len() * 9);
    for account in resp.account_statz {
        let tags = tags!(
            "account" => account.acc.clone(),
            "account_id" => account.acc,
            "account_name" => account.name,
        );
        metrics.extend([
            Metric::gauge_with_tags(
                "gnatds_accstatz_current_connections",
                "",
                account.conns,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "gnatds_accstatz_total_connections",
                "",
                account.total_conns,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "gnatds_accstatz_subscriptions",
                "",
                account.num_subscriptions,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "gnatds_accstatz_leaf_nodes",
                "",
                account.leaf_nodes,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "gnatds_accstatz_sent_messages",
                "",
                account.sent.msgs,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "gnatds_accstatz_sent_bytes",
                "",
                account.sent.bytes,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "gnatds_accstatz_received_messages",
                "",
                account.received.msgs,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "gnatds_accstatz_received_bytes",
                "",
                account.received.bytes,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "gnatds_accstatz_slow_consumers",
                "",
                account.slow_consumers,
                tags,
            ),
        ]);
    }

    Ok(metrics)
}
