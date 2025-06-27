use std::time::Duration;

use bytes::Bytes;
use event::Event;
use framework::config::{ProxyConfig, SinkConfig, SinkContext};
use framework::http::HttpClient;
use http_body_util::{BodyExt, Full};
use serde::Deserialize;
use testify::container::Container;
use testify::next_addr;
use testify::random::random_string;
use testify::wait::wait_for_duration;
use url::Url;

use super::config::{Config, healthcheck};
use crate::testing::trace_init;

#[derive(Debug, Deserialize)]
struct QueryResp {
    status: String,
    data: Vec<String>,
}

#[tokio::test]
async fn write_and_query() {
    trace_init();

    let service_addr = next_addr();

    Container::new("grafana/loki", "3.0.1")
        .with_tcp(3100, service_addr.port())
        .tail_logs(false, true)
        .run(async move {
            // wait for loki get ready
            wait_for_duration(
                async move || {
                    let url = Url::parse(&format!("http://{service_addr}")).unwrap();
                    let client = HttpClient::new(None, &ProxyConfig::default()).unwrap();

                    healthcheck(url, None, client).await.is_ok()
                },
                Duration::from_secs(30),
            )
            .await;

            // setup loki sink
            info!(message = "start sink", ?service_addr);

            let label_value = random_string(8);
            let config = format!(
                r#"
encoding:
  codec: json
endpoint: http://{}
labels:
  foo: {}
    "#,
                service_addr, &label_value
            );

            let config: Config = serde_yaml::from_str(&config).unwrap();
            let cx = SinkContext::new_test();

            let (sink, healthcheck) = config.build(cx).await.unwrap();

            tokio::time::sleep(Duration::from_secs(5)).await;

            healthcheck.await.unwrap();

            sink.run_events(vec![Event::from("some log")])
                .await
                .unwrap();

            // wait until all events flushed
            tokio::time::sleep(Duration::from_secs(2)).await;

            // Query label values
            let endpoint = format!("http://{service_addr}/loki/api/v1/label/foo/values");
            let client: HttpClient<Full<Bytes>> =
                HttpClient::new(None, &ProxyConfig::default()).unwrap();
            let req = http::Request::get(endpoint).body(Full::default()).unwrap();
            let resp = client.send(req).await.unwrap();

            // Assert
            let (parts, incoming) = resp.into_parts();
            assert!(parts.status.is_success());

            let body = incoming.collect().await.unwrap().to_bytes();
            let qr: QueryResp = serde_json::from_slice(body.as_ref()).unwrap();
            assert_eq!(qr.status, "success".to_string());
            assert_eq!(qr.data.len(), 1);
            assert_eq!(qr.data[0], label_value);
        })
        .await
}
