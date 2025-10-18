use event::{Metric, tags};
use framework::http::HttpClient;
use serde::Deserialize;

use super::{Error, fetch};

#[derive(Deserialize)]
struct Healthz {
    status: String,
}

pub async fn collect(client: &HttpClient, endpoint: &str) -> Result<Vec<Metric>, Error> {
    let resp = match fetch::<Healthz>(client, &format!("{endpoint}/healthz")).await {
        Ok(resp) => resp,
        Err(_err) => {
            return Ok(vec![
                Metric::gauge("gnatsd_healthz_status", "", 0),
                Metric::gauge_with_tags(
                    "gnatsd_healthz_status_value",
                    "",
                    1,
                    tags!(
                        "status" => "unreachable",
                    ),
                ),
            ]);
        }
    };

    Ok(vec![
        Metric::gauge("gnatsd_healthz_status", "", resp.status == "ok"),
        Metric::gauge_with_tags(
            "gnatsd_healthz_status_value",
            "",
            1,
            tags!(
                "status" => resp.status.as_str(),
            ),
        ),
    ])
}
