use framework::http::HttpClient;

use crate::sinks::loki::config::Config;

pub async fn health_check(config: Config, client: HttpClient) -> crate::Result<()> {
    let endpoint = config.endpoint.append_path("ready")?;
    let mut req = http::Request::get(endpoint.uri)
        .body(hyper::Body::empty())
        .expect("Building request never fails");

    if let Some(auth) = &config.auth {
        auth.apply(&mut req);
    }

    let resp = client.send(req).await?;
    match resp.status() {
        http::StatusCode::OK => Ok(()),
        _ => Err(format!("A non-successful status returned: {}", resp.status()).into()),
    }
}
