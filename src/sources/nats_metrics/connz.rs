use event::{Metric, tags};
use framework::http::HttpClient;
use serde::Deserialize;

use super::{Error, fetch};

#[derive(Deserialize)]
struct Connection {
    pending_bytes: usize,
    subscriptions: usize,
    in_bytes: usize,
    out_bytes: usize,
    in_msgs: usize,
    out_msgs: usize,
}

#[derive(Deserialize)]
struct Connz {
    num_connections: usize,
    total: usize,
    offset: usize,
    limit: usize,
    connections: Vec<Connection>,
}

pub async fn collect(client: &HttpClient, endpoint: &str) -> Result<Vec<Metric>, Error> {
    let uri = format!("{endpoint}/connz");
    let connz = fetch::<Connz>(client, &uri).await?;

    let mut pending_bytes = 0;
    let mut subscriptions = 0;
    let mut in_bytes = 0;
    let mut out_bytes = 0;
    let mut in_msgs = 0;
    let mut out_msgs = 0;
    for conn in connz.connections {
        pending_bytes += conn.pending_bytes;
        subscriptions += conn.subscriptions;
        in_bytes += conn.in_bytes;
        out_bytes += conn.out_bytes;
        in_msgs += conn.in_msgs;
        out_msgs += conn.out_msgs;

        // todo: detailed
    }

    let tags = tags!();

    Ok(vec![
        Metric::gauge_with_tags(
            "gnatsd_connz_num_connections",
            "Number of connections",
            connz.num_connections,
            tags.clone(),
        ),
        Metric::gauge_with_tags("gnatsd_connz_total", "", connz.total, tags.clone()),
        Metric::gauge_with_tags("gnatsd_connz_offset", "", connz.offset, tags.clone()),
        Metric::gauge_with_tags("gnatsd_connz_limit", "", connz.limit, tags.clone()),
        Metric::gauge_with_tags(
            "gnatsd_connz_pending_bytes",
            "",
            pending_bytes,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "gnatsd_connz_subscriptions",
            "",
            subscriptions,
            tags.clone(),
        ),
        Metric::sum_with_tags("gnatsd_connz_in_bytes", "", in_bytes, tags.clone()),
        Metric::sum_with_tags("gnatsd_connz_out_bytes", "", out_bytes, tags.clone()),
        Metric::sum_with_tags("gnatsd_connz_in_msgs", "", in_msgs, tags.clone()),
        Metric::sum_with_tags("gnatsd_connz_out_msgs", "", out_msgs, tags),
    ])
}
