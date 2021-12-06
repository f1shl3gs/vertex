use crate::http::HttpClient;
use crate::sinks::loki::config::LokiConfig;

async fn fetch_status(
    endpoint: &str,
    config: &LokiConfig,
    client: &HttpClient,
) -> crate::Result<http::StatusCode> {
    let mut req = http::Request::get(endpoint)
        .body(hyper::Body::empty())
        .unwrap();

    if let Some(auth) = &config.auth {
        auth.apply(&mut req);
    }

    Ok(client.send(req).await?.status())
}

pub async fn health_check(config: LokiConfig, client: HttpClient) -> crate::Result<()> {
    let mut req = http::Request::get("")
        .body(hyper::Body::empty())
        .unwrap();

    if let Some(auth) = &config.auth {
        auth.apply(&mut req);
    }

    let resp = client.send(req).await?;
    let status = match fetch_status("ready", &config, &client).await? {
        http::StatusCode::NOT_FOUND => {
            debug!(
                message = "Endpoint `/ready` not found. Retrying healthcheck with top level query"
            );

            fetch_status("", &config, &client).await?
        }

        status => status
    };

    match status {
        http::StatusCode::OK => Ok(()),
        _ => Err(format!(
            "A non-successful status returned: {}",
            resp.status()
        ).into()),
    }
}