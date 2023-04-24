use std::io::read_to_string;

use bytes::Buf;
use event::{BatchNotifier, BatchStatus, BatchStatusReceiver, Event, LogRecord};
use framework::batch::BatchConfig;
use framework::config::{ProxyConfig, SinkConfig, SinkContext};
use framework::http::HttpClient;
use framework::sink::util::service::RequestConfig;
use framework::sink::util::Compression;
use framework::tls::TlsSettings;
use futures_util::future::ready;
use futures_util::stream;
use http::{Method, Request};
use hyper::Body;
use serde::Deserialize;
use serde_json::Value;
use testify::random::random_string;

use crate::sinks::clickhouse::config::Config;
use crate::testing::components::{run_and_assert_sink_compliance, HTTP_SINK_TAGS};
use crate::testing::trace_init;

fn clickhouse_address() -> String {
    "http://localhost:8123".to_string()
}

fn gen_table() -> String {
    format!("test_{}", random_string(10).to_lowercase())
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // deserialize all fields
struct Stats {
    bytes_read: usize,
    elapsed: f64,
    rows_read: usize,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // deserialize all fields
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
        let client = HttpClient::new(&None, &ProxyConfig::default()).unwrap();

        Self { host, client }
    }

    async fn create_table(&self, table: &str, schema: &str) {
        let req = Request::builder()
            .method(Method::POST)
            .uri(&self.host)
            .body(Body::from(format!(
                "CREATE TABLE {}
                    ({})
                    ENGINE = MergeTree()
                    ORDER BY (host, timestamp);",
                table, schema
            )))
            .unwrap();

        let resp = self.client.send(req).await.unwrap();
        let (parts, body) = resp.into_parts();

        if !parts.status.is_success() {
            let reader = hyper::body::aggregate(body).await.unwrap().reader();
            let body = read_to_string(reader).unwrap();
            panic!("create table failed, {}", body)
        }
    }

    async fn select_all(&self, table: &str) -> QueryResponse {
        let req = Request::builder()
            .method(Method::POST)
            .uri(&self.host)
            .body(Body::from(format!("SELECT * FROM {} FORMAT JSON", table)))
            .unwrap();

        let resp = self.client.send(req).await.unwrap();
        let (parts, body) = resp.into_parts();
        let reader = hyper::body::aggregate(body).await.unwrap().reader();
        let body = read_to_string(reader).unwrap();

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
    let mut event = LogRecord::from("raw log line").with_batch_notifier(&batch);
    event.insert_tag("host", "example.com");
    (event.into(), receiver)
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
            retry_attempts: Some(1),
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
        .insert_field("items", vec!["item1", "item2"]);

    run_and_assert_sink_compliance(
        sink,
        stream::once(ready(input_event.clone())),
        &HTTP_SINK_TAGS,
    )
    .await;

    let output = client.select_all(&table).await;
    assert_eq!(1, output.rows);

    let expected = serde_json::to_value(input_event.into_log()).unwrap();
    assert_eq!(expected, output.data[0]);

    assert_eq!(receiver.try_recv(), Ok(BatchStatus::Delivered));
}
