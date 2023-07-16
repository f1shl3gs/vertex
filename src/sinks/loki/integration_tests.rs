use std::time::Duration;

use crate::testing::{ContainerBuilder, WaitFor};
use event::Event;
use framework::config::{ProxyConfig, SinkConfig, SinkContext};
use framework::http::HttpClient;
use hyper::Body;
use serde::{Deserialize, Serialize};
use testify::random::random_string;

use super::config::LokiConfig;

const LOKI_PORT: u16 = 3100;

#[tokio::test]
async fn write_and_query() {
    // 1. setup Loki all-in-one container
    let container = ContainerBuilder::new("grafana/loki:1.4.1")
        .port(LOKI_PORT)
        .run()
        .unwrap();
    container.wait(WaitFor::Stderr("Starting Loki")).unwrap();
    let address = container.get_host_port(LOKI_PORT).unwrap();

    // 2. setup loki service
    let label_value = random_string(8);
    let config = format!(
        r#"
compression: none
encoding:
  codec: json
endpoint: http://{}
labels:
  foo: {}
    "#,
        address, &label_value
    );

    let config: LokiConfig = serde_yaml::from_str(&config).unwrap();
    let cx = SinkContext::new_test();

    let (sink, healthcheck) = config.build(cx).await.unwrap();
    healthcheck.await.unwrap();

    sink.run_events(vec![Event::from("some log")])
        .await
        .unwrap();

    // wait until all events flushed
    tokio::time::sleep(Duration::from_secs(2)).await;

    // 3. Query label values
    let endpoint = format!("http://{}/loki/api/v1/label/foo/values", address);
    let client = HttpClient::new(&None, &ProxyConfig::default()).unwrap();
    let req = http::Request::get(endpoint).body(Body::empty()).unwrap();
    let resp = client.send(req).await.unwrap();

    // 4. Assert
    #[derive(Debug, Deserialize, Serialize)]
    struct QueryResp {
        status: String,
        data: Vec<String>,
    }

    let (parts, body) = resp.into_parts();
    assert!(parts.status.is_success());
    let body = hyper::body::to_bytes(body).await.unwrap();
    let qr: QueryResp = serde_json::from_slice(body.as_ref()).unwrap();
    assert_eq!(qr.status, "success".to_string());
    assert_eq!(qr.data.len(), 1);
    assert_eq!(qr.data[0], label_value);
}
