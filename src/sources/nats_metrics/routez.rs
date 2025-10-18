use event::Metric;
use framework::http::HttpClient;
use serde::Deserialize;

use super::{Error, fetch};

#[derive(Deserialize)]
struct Routez {
    num_routes: u64,
}

pub async fn collect(client: &HttpClient, endpoint: &str) -> Result<Vec<Metric>, Error> {
    let resp = fetch::<Routez>(client, &format!("{endpoint}/routez")).await?;

    Ok(vec![Metric::gauge(
        "gnatsd_routez_routes_total",
        "Number of routes in GNATS",
        resp.num_routes,
    )])
}
