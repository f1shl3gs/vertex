use std::time::Duration;

use crate::sinks::elasticsearch::config::ElasticsearchMode;
use bytes::Bytes;
use chrono::TimeZone;
use chrono::Utc;
use event::{fields, BatchNotifier, BatchStatus, Events, LogRecord};
use framework::batch::BatchConfig;
use framework::config::{ProxyConfig, SinkConfig, SinkContext};
use framework::http::HttpClient;
use framework::sink::util::sink::Response;
use framework::sink::util::Compression;
use framework::HealthcheckError;
use futures_util::future::ready;
use futures_util::stream;
use http::{Request, StatusCode};
use log_schema::log_schema;
use serde_json::json;
use testcontainers::core::WaitFor;
use testcontainers::images::generic::GenericImage;
use testify::random::random_string;
use tokio::time::sleep;

use super::common::ElasticsearchCommon;
use super::config::{BulkConfig, ElasticsearchConfig};

impl ElasticsearchCommon {
    async fn flush_request(&self) -> crate::Result<()> {
        let url = format!("{}/_flush", self.base_url)
            .parse::<hyper::Uri>()
            .unwrap();
        let mut builder = Request::post(&url);

        if let Some(ce) = self.request_builder.compression.content_encoding() {
            builder = builder.header("Content-Encoding", ce);
        }

        for (k, v) in &self.request.headers {
            builder = builder.header(&k[..], &v[..]);
        }

        if let Some(auth) = &self.http_auth {
            builder = auth.apply_builder(builder);
        }

        let req = builder.body(Bytes::new())?;
        let proxy = ProxyConfig::default();
        let client = HttpClient::new(self.tls_settings.clone(), &proxy)
            .expect("Could not build client to flush");
        let resp = client.send(req.map(hyper::Body::from)).await?;

        match resp.status() {
            StatusCode::OK => Ok(()),
            status => Err(HealthcheckError::UnexpectedStatus(status).into()),
        }
    }

    async fn query(&self, base_url: &str, index: &str) -> crate::Result<serde_json::Value> {
        let mut builder = Request::get(format!("{}/{}/_search", base_url, index));
        if let Some(ce) = self.request_builder.compression.content_encoding() {
            builder = builder.header("Content-Encoding", ce);
        }

        for (k, v) in &self.request.headers {
            builder = builder.header(&k[..], &v[..]);
        }

        if let Some(auth) = &self.http_auth {
            builder = auth.apply_builder(builder);
        }

        builder = builder.header("Content-Type", "application/json");
        let req = builder.body(Bytes::from(r#"{"query":{"query_string":{"query":"*"}}}"#))?;
        let proxy = ProxyConfig::default();
        let client = HttpClient::new(self.tls_settings.clone(), &proxy)
            .expect("Could not build client to query");
        let resp = client.send(req.map(hyper::Body::from)).await?;

        assert!(resp.is_successful());

        let body = hyper::body::to_bytes(resp.into_body()).await?;
        serde_json::from_slice(&body).map_err(Into::into)
    }
}

fn gen_index() -> String {
    format!("test-{}", random_string(10).to_lowercase())
}

fn config() -> ElasticsearchConfig {
    let mut batch = BatchConfig::default();
    batch.max_events = Some(1);

    ElasticsearchConfig {
        batch,
        ..Default::default()
    }
}

async fn flush(common: ElasticsearchCommon) -> crate::Result<()> {
    sleep(Duration::from_secs(2)).await;
    common.flush_request().await?;
    sleep(Duration::from_secs(2)).await;

    Ok(())
}

#[tokio::test]
async fn ensure_pipeline_in_params() {
    let index = gen_index();
    let pipeline = "test-pipeline".to_string();

    let config = ElasticsearchConfig {
        endpoint: "http://example.com".into(),
        bulk: Some(BulkConfig {
            index: Some(index),
            action: None,
        }),
        pipeline: Some(pipeline.clone()),
        ..config()
    };

    let common = ElasticsearchCommon::parse_config(&config)
        .await
        .expect("Config error");

    assert_eq!(common.query_params["pipeline"], pipeline)
}

#[tokio::test]
async fn structures_events_correctly() {
    let tag = "7.17.5";
    let image = GenericImage::new("elasticsearch", tag)
        .with_env_var("discovery.type", "single-node")
        .with_wait_for(WaitFor::StdOutMessage {
            message: "Cluster health status changed from [YELLOW] to [GREEN]".to_string(),
        });

    let cli = testcontainers::clients::Cli::default();
    let container = cli.run(image);
    let host_port = container.get_host_port_ipv4(9200);
    let endpoint = format!("http://localhost:{}", host_port);

    let index = gen_index();
    let config = ElasticsearchConfig {
        endpoint,
        bulk: Some(BulkConfig {
            index: Some(index.clone()),
            action: None,
        }),
        doc_type: Some("log_lines".into()),
        id_key: Some("my_id".into()),
        compression: Compression::None,
        ..config()
    };
    let common = ElasticsearchCommon::parse_config(&config)
        .await
        .expect("Config error");
    let base_url = common.base_url.clone();
    let cx = SinkContext::new_test();
    let (sink, hc) = config.build(cx.clone()).await.unwrap();
    hc.await.expect("Health check failed");

    let (batch, mut receiver) = BatchNotifier::new_with_receiver();
    let input = LogRecord::from(fields!(
        log_schema().message_key() => "raw log line",
        log_schema().timestamp_key() => Utc.ymd(2020, 12, 1).and_hms(1, 2, 3),
        "my_id" => "42",
        "foo" => "bar"
    ))
    .with_batch_notifier(&batch);

    drop(batch);

    let timestamp = input
        .get_field(log_schema().timestamp_key())
        .unwrap()
        .clone();
    let events = stream::once(ready(Events::from(input)));
    sink.run(events).await.expect("Running sink failed");

    assert_eq!(receiver.try_recv(), Ok(BatchStatus::Delivered));

    // make sure writes all visible
    flush(common.clone()).await.expect("Could not flush");

    let resp = common
        .query(base_url.as_str(), index.as_str())
        .await
        .expect("Could not query");

    let total = resp["hits"]["total"]
        .as_u64()
        .or_else(|| resp["hits"]["total"]["value"].as_u64())
        .expect("Elasticsearch response does not include hits->total nor hits->total->value");
    assert_eq!(1, total);

    let hits = resp["hits"]["hits"]
        .as_array()
        .expect("Elasticsearch resp does not include hits->hits");

    let hit = hits.iter().next().unwrap();
    assert_eq!("42", hit["_id"]);

    let value = hit
        .get("_source")
        .expect("Elasticsearch hit missing _source");
    assert_eq!(None, value["some_id"].as_str());

    let expected = json!({
        "message": "raw log line",
        "foo": "bar",
        "timestamp": timestamp
    });
    assert_eq!(&expected, value)
}

async fn run_insert_tests(
    mut config: ElasticsearchConfig,
    break_events: bool,
    status: BatchStatus,
) {
    config.bulk = Some(BulkConfig {
        index: Some(gen_index()),
        action: None,
    });
    run_insert_tests_with_config(&config, break_events, status).await;
}

async fn run_insert_tests_with_config(
    config: &ElasticsearchConfig,
    break_events: bool,
    batch_status: BatchStatus,
) {
    let common = ElasticsearchCommon::parse_config(config)
        .await
        .expect("Config error");
    let index = match config.mode {
        ElasticsearchMode::DataStream => format!(
            "{}",
            Utc::now().format(".ds-logs-generic-default-%Y.%m.%d-000001")
        ),
        ElasticsearchMode::Bulk => config
            .bulk
            .as_ref()
            .map(|x| x.index.clone().unwrap())
            .unwrap(),
    };
    let base_url = common.base_url.clone();

    let cx = SinkContext::new_test();
    let (sink, healthcheck) = config
        .build(cx.clone())
        .await
        .expect("Building config failed");

    healthcheck.await.expect("Healthcheck failed");

    let (batch, mut receiver) = BatchNotifier::new_with_receiver();
}

#[tokio::test]
async fn insert_events_over_http() {
    run_in
}
