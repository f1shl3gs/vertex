use bytes::Bytes;
use framework::HealthcheckError;
use framework::http::HttpClient;
use http::{HeaderValue, Method, Request, StatusCode};
use http_body_util::{BodyExt, Full};
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
    let mut req = Request::builder()
        .method(Method::GET)
        .uri(format!("{endpoint}/health"))
        .body(Full::<Bytes>::default())?;

    // Authorization: Token INFLUX_API_TOKEN
    req.headers_mut().insert(
        "Authorization",
        HeaderValue::from_str(&format!("Token {token}")).unwrap(),
    );

    let resp = client.send(req).await?;
    let (parts, incoming) = resp.into_parts();
    let data = incoming.collect().await?.to_bytes();

    if parts.status != StatusCode::OK {
        return Err(HealthcheckError::UnexpectedStatus(
            parts.status,
            String::from_utf8_lossy(&data).to_string(),
        )
        .into());
    }

    let resp: HealthResponse = serde_json::from_slice(&data)?;

    if resp.status == "pass" {
        Ok(())
    } else {
        Err(resp.message.into())
    }
}
