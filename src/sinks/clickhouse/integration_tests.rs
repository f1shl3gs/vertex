use bytes::Bytes;
use event::{BatchNotifier, BatchStatus, BatchStatusReceiver, Event, LogRecord};
use framework::batch::BatchConfig;
use framework::config::{ProxyConfig, SinkConfig, SinkContext};
use framework::http::HttpClient;
use framework::sink::util::Compression;
use framework::sink::util::service::RequestConfig;
use futures::future::ready;
use http::{Method, Request};
use http_body_util::{BodyExt, Full};
use serde::Deserialize;
use serde_json::Value;
use testify::random::random_string;

use super::config::Config;
use crate::testing::components::{HTTP_SINK_TAGS, run_and_assert_sink_compliance};
use crate::testing::trace_init;

fn clickhouse_address() -> String {
    "http://localhost:8123".to_string()
}

fn gen_table() -> String {
    format!("test_{}", random_string(10).to_lowercase())
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct Stats {
    bytes_read: usize,
    elapsed: f64,
    rows_read: usize,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct QueryResponse {
    data: Vec<Value>,
    meta: Vec<Value>,
    rows: usize,
    statistics: Stats,
}

struct ClickhouseClient {
    host: String,
    client: HttpClient,
}

impl ClickhouseClient {
    fn new(host: String) -> Self {
        let client = HttpClient::new(None, &ProxyConfig::default()).unwrap();

        Self { host, client }
    }

    async fn create_table(&self, table: &str, schema: &str) {
        let req = Request::builder()
            .method(Method::POST)
            .uri(&self.host)
            .body(Full::new(Bytes::from(format!(
                "CREATE TABLE {}
                    ({})
                    ENGINE = MergeTree()
                    ORDER BY (host, timestamp);",
                table, schema
            ))))
            .unwrap();

        let resp = self.client.send(req).await.unwrap();
        let (parts, incoming) = resp.into_parts();

        if !parts.status.is_success() {
            let data = incoming.collect().await.unwrap().to_bytes();
            let body = String::from_utf8_lossy(data.as_ref());
            panic!("create table failed, {}", body)
        }
    }

    async fn select_all(&self, table: &str) -> QueryResponse {
        let req = Request::builder()
            .method(Method::POST)
            .uri(&self.host)
            .body(Full::new(Bytes::from(format!(
                "SELECT * FROM {} FORMAT JSON",
                table
            ))))
            .unwrap();

        let resp = self.client.send(req).await.unwrap();
        let (parts, incoming) = resp.into_parts();
        let data = incoming.collect().await.unwrap().to_bytes();
        let body = String::from_utf8_lossy(data.as_ref());

        if !parts.status.is_success() {
            panic!("select all failed, {}", body);
        } else {
            match serde_json::from_str(&body) {
                Ok(value) => value,
                Err(err) => panic!("unmarshal resp failed, {}", err),
            }
        }
    }
}

fn make_event() -> (Event, BatchStatusReceiver) {
    let (batch, receiver) = BatchNotifier::new_with_receiver();
    let mut log = LogRecord::from("raw log line").with_batch_notifier(&batch);
    log.insert("host", "example.com");
    (log.into(), receiver)
}

#[tokio::test]
async fn insert_events() {
    trace_init();

    let table = gen_table();
    let host = clickhouse_address();

    let mut batch = BatchConfig::default();
    batch.max_events = Some(1);

    let conf = Config {
        endpoint: host.parse().unwrap(),
        table: table.clone(),
        database: "default".to_string(),
        skip_unknown_fields: false,
        date_time_best_effort: false,
        compression: Compression::None,
        encoding: Default::default(),
        batch,
        auth: None,
        request: RequestConfig {
            retry_attempts: 1,
            ..Default::default()
        },
        tls: None,
        acknowledgements: false,
    };

    let client = ClickhouseClient::new(host);
    client
        .create_table(
            &table,
            "host String, timestamp String, message String, items Array(String)",
        )
        .await;

    let (sink, hc) = conf.build(SinkContext::new_test()).await.unwrap();

    hc.await.expect("healthcheck pass");

    let (mut input_event, mut receiver) = make_event();
    input_event
        .as_mut_log()
        .insert("items", vec!["item1", "item2"]);

    run_and_assert_sink_compliance(
        sink,
        futures::stream::once(ready(input_event.clone())),
        &HTTP_SINK_TAGS,
    )
    .await;

    let output = client.select_all(&table).await;
    assert_eq!(1, output.rows);

    let expected = serde_json::to_value(input_event.into_log()).unwrap();
    assert_eq!(expected, output.data[0]);

    assert_eq!(receiver.try_recv(), Ok(BatchStatus::Delivered));
}
