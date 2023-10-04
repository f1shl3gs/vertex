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
