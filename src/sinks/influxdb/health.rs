use framework::http::HttpClient;
use framework::HealthcheckError;
use http::{HeaderValue, Request, StatusCode};
use hyper::Body;
use serde::Deserialize;

#[derive(Deserialize)]
struct HealthResponse {
    message: String,
    // enum: pass or fail
    status: String,
}

/// Issue a health check request
///
/// See https://docs.influxdata.com/influxdb/v2/api/#operation/GetHealth
pub async fn healthcheck(client: HttpClient, endpoint: String, token: String) -> crate::Result<()> {
    // Authorization: Token INFLUX_API_TOKEN
    let uri = format!("{}/health", endpoint);
    let mut req = Request::get(uri).body(Body::empty())?;
    req.headers_mut().insert(
        "Authorization",
        HeaderValue::from_str(&format!("Token {}", token)).unwrap(),
    );

    let resp = client.send(req).await?;

    let (parts, body) = resp.into_parts();
    match parts.status {
        StatusCode::OK => {
            let data = hyper::body::to_bytes(body).await?;
            let resp: HealthResponse = serde_json::from_slice(&data)?;
            if resp.status == "pass" {
                Ok(())
            } else {
                Err(resp.message.into())
            }
        }
        status => Err(HealthcheckError::UnexpectedStatus(status).into()),
    }
}

#[cfg(test)]
mod tests {
    use framework::config::ProxyConfig;

    use super::*;

    #[tokio::test]
    async fn health_check() {
        let endpoint = "http://localhost:8086".to_string();
        let token = "5rGWQ9YT5mD2FqKP4-WcOfLS232LWj8OWwc9dCH0Jy6QRu9ckjj3eL4S5Mwjzd5ZI4Z82SCnpKI-XKI9KoF1gw==".to_string();
        let client = HttpClient::new(&None, &ProxyConfig::default()).unwrap();
        healthcheck(client, endpoint, token).await.unwrap();
    }
}
