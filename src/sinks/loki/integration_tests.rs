use bytes::Bytes;
use event::Event;
use framework::config::{ProxyConfig, SinkConfig, SinkContext};
use framework::http::HttpClient;
use http_body_util::{BodyExt, Full};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use testify::random::random_string;

use super::config::Config;
use crate::testing::{ContainerBuilder, WaitFor};

const LOKI_PORT: u16 = 3100;

#[tokio::test]
async fn write_and_query() {
    // 1. setup Loki all-in-one container
    let container = ContainerBuilder::new("grafana/loki:1.4.1")
        .with_port(LOKI_PORT)
        .run()
        .unwrap();
    container.wait(WaitFor::Stderr("Starting Loki")).unwrap();
    let address = container.get_mapped_addr(LOKI_PORT);

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

    let config: Config = serde_yaml::from_str(&config).unwrap();
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
    let client: HttpClient<Full<Bytes>> = HttpClient::new(None, &ProxyConfig::default()).unwrap();
    let req = http::Request::get(endpoint).body(Full::default()).unwrap();
    let resp = client.send(req).await.unwrap();

    // 4. Assert
    #[derive(Debug, Deserialize, Serialize)]
    struct QueryResp {
        status: String,
        data: Vec<String>,
    }

    let (parts, incoming) = resp.into_parts();
    assert!(parts.status.is_success());
    let body = incoming.collect().await.unwrap().to_bytes();
    let qr: QueryResp = serde_json::from_slice(body.as_ref()).unwrap();
    assert_eq!(qr.status, "success".to_string());
    assert_eq!(qr.data.len(), 1);
    assert_eq!(qr.data[0], label_value);
}
