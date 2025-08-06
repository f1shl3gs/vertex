use std::time::Duration;

use bytes::Bytes;
use framework::config::ProxyConfig;
use framework::http::HttpClient;
use framework::tls::TlsConfig;
use http::header::AUTHORIZATION;
use http::{HeaderValue, Request, StatusCode};
use http_body_util::{BodyExt, Full};
use tokio::time::Interval;

use super::pod::{Pod, PodList};

const SERVICE_ACCOUNT_TOKEN: &str = "/var/run/secrets/kubernetes.io/serviceaccount/token";
const SERVICE_CERTFILE: &str = "/var/run/secrets/kubernetes.io/serviceaccount/ca.crt";

pub struct KubeletProvider {
    endpoint: String,
    client: HttpClient,

    interval: Interval,
}

impl KubeletProvider {
    pub fn new(endpoint: Option<&String>, interval: Duration) -> Result<Self, crate::Error> {
        let endpoint = match endpoint {
            Some(value) => format!("https://{value}"),
            None => match std::env::var("NODE_NAME") {
                Ok(name) => format!("https://{name}:10250"),
                Err(_err) => return Err("default environment variable `NODE_NAME` not set".into()),
            },
        };

        let tls = TlsConfig {
            ca: Some(SERVICE_CERTFILE.into()),
            verify_certificate: false,
            verify_hostname: false,
            ..Default::default()
        };
        let client = HttpClient::new(Some(&tls), &ProxyConfig::default())?;

        Ok(Self {
            client,
            endpoint,
            interval: tokio::time::interval(interval),
        })
    }

    pub async fn list(&mut self) -> std::io::Result<Vec<Pod>> {
        let pods = loop {
            self.interval.tick().await;

            // load service account token
            let token = match std::fs::read_to_string(SERVICE_ACCOUNT_TOKEN) {
                Ok(token) => token,
                Err(err) => {
                    warn!(message = "failed to read service account token", ?err);
                    continue;
                }
            };

            let Ok(auth) = HeaderValue::from_str(&format!("Bearer {}", token.trim())) else {
                warn!(message = "authorization token header build failed",);
                continue;
            };

            let req = Request::get(format!("{}/pods", self.endpoint))
                .header(AUTHORIZATION, auth)
                .body(Full::<Bytes>::default())
                .unwrap();

            match tokio::time::timeout(Duration::from_secs(5), self.client.send(req)).await {
                Ok(Ok(resp)) => {
                    let (parts, incoming) = resp.into_parts();
                    let data = match incoming.collect().await {
                        Ok(data) => data.to_bytes(),
                        Err(err) => {
                            warn!(message = "read pods response failed", ?err);

                            continue;
                        }
                    };

                    if parts.status != StatusCode::OK {
                        warn!(
                            message = "invalid pods response",
                            status = ?parts.status,
                            body = String::from_utf8_lossy(&data).as_ref()
                        );

                        continue;
                    }

                    match serde_json::from_slice::<PodList>(&data) {
                        Ok(pods) => break pods,
                        Err(err) => {
                            warn!(message = "decode pod list failed", ?err);

                            continue;
                        }
                    }
                }
                Ok(Err(err)) => {
                    warn!(message = "fetch pods failed", ?err);
                    continue;
                }
                Err(_) => {
                    warn!(message = "fetch pods timeout");
                    continue;
                }
            }
        };

        Ok(pods.items)
    }
}
