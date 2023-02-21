use std::future::Future;
use std::time::Duration;

use bytes::Bytes;
use chrono::TimeZone;
use chrono::Utc;
use event::{fields, BatchNotifier, BatchStatus, Events, LogRecord};
use framework::batch::BatchConfig;
use framework::config::{ProxyConfig, SinkConfig, SinkContext};
use framework::http::HttpClient;
use framework::sink::util::sink::Response;
use framework::sink::util::Compression;
use framework::{tls::TlsConfig, HealthcheckError};
use futures::StreamExt;
use futures_util::future::ready;
use futures_util::stream;
use http::{Request, StatusCode};
use log_schema::log_schema;
use serde_json::json;
use testcontainers::core::WaitFor;
use testcontainers::images::generic::GenericImage;
use testify::random::{random_events_with_stream, random_string};
use tokio::time::sleep;

use super::common::ElasticsearchCommon;
use super::config::{
    BulkConfig, ElasticsearchAuth, ElasticsearchConfig, ElasticsearchMode,
    DATA_STREAM_TIMESTAMP_KEY,
};
use crate::testing::trace_init;

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

#[test]
fn structures_events_correctly() {
    setup_and_run(|endpoint| {
        async move {
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
                log_schema().timestamp_key() => Utc.with_ymd_and_hms(2020, 12, 1, 1, 2, 3).unwrap(),
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
                .expect(
                    "Elasticsearch response does not include hits->total nor hits->total->value",
                );
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
    })
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
    let (input, events) = random_events_with_stream(100, 100, Some(batch));
    if break_events {
        // Break all but the first event to simulate some kind of partial failure
        let mut doit = false;
        let events = events.map(move |mut events| {
            if doit {
                events.for_each_log(|log| {
                    log.insert_field("_type", 1);
                });
            }

            doit = true;
            events
        });

        sink.run(events.map(Into::into))
            .await
            .expect("Running sink failed");
    } else {
        sink.run(events.map(Into::into))
            .await
            .expect("Running sink failed");
    };

    assert_eq!(receiver.try_recv(), Ok(batch_status));

    // make sure writes all visible
    flush(common.clone()).await.expect("Flushing writes failed");

    let mut resp = common.query(&base_url, &index).await.expect("Query failed");
    let total = resp["hits"]["total"]["value"]
        .as_u64()
        .or_else(|| resp["hits"]["total"].as_u64())
        .expect("Elasticsearch response does not include hits->total nor hits->total->value");

    if break_events {
        assert_ne!(input.len() as u64, total);
    } else {
        assert_eq!(input.len() as u64, total);

        let hits = resp["hits"]["hits"]
            .as_array_mut()
            .expect("Elasticsearch response does not include hits->hits");
        #[allow(clippy::needless_collect)] // https://github.com/rust-lang/rust-clippy/issues/6909
        let input = input
            .into_iter()
            .map(|rec| serde_json::to_value(&rec.into_log().fields).unwrap())
            .collect::<Vec<_>>();

        for hit in hits {
            let hit = hit
                .get_mut("_source")
                .expect("Elasticsearch hit missing _source");
            if config.mode == ElasticsearchMode::DataStream {
                let obj = hit.as_object_mut().unwrap();
                obj.remove("data_stream");

                // Un-rewrite the timestamp field
                let timestamp = obj.remove(DATA_STREAM_TIMESTAMP_KEY).unwrap();
                obj.insert(log_schema().timestamp_key().into(), timestamp);
            }

            assert!(input.contains(hit));
        }
    }
}

#[test]
fn insert_events_over_http() {
    setup_and_run(|endpoint| async move {
        run_insert_tests(
            ElasticsearchConfig {
                endpoint,
                doc_type: Some("log_lines".into()),
                compression: Compression::None,
                ..config()
            },
            false,
            BatchStatus::Delivered,
        )
        .await
    })
}

#[tokio::test]
async fn insert_events_over_https() {
    let tag = "7.17.5";
    let pwd = std::env::current_dir()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let image = GenericImage::new("elasticsearch", tag)
        .with_env_var("discovery.type", "single-node")
        .with_env_var("ingest.geoip.downloader.enabled", "false")
        .with_env_var("ES_JAVA_OPTS", "-Xms512m -Xmx512m")
        .with_env_var("ELASTIC_PASSWORD", "password")
        // setup tls
        .with_volume(
            format!("{}/tests/ca", pwd),
            "/usr/share/elasticsearch/config/certs",
        )
        .with_env_var("xpack.security.enabled", "true")
        .with_env_var("xpack.security.http.ssl.enabled", "true")
        .with_env_var(
            "xpack.security.http.ssl.certificate",
            "certs/intermediate_server/certs/elasticsearch-secure-chain.cert.pem",
        )
        .with_env_var(
            "xpack.security.http.ssl.key",
            "certs/intermediate_server/private/elasticsearch-secure.key.pem",
        )
        .with_env_var("xpack.security.transport.ssl.enabled", "true")
        .with_env_var(
            "xpack.security.transport.ssl.certificate",
            "certs/intermediate_server/certs/elasticsearch-secure-chain.cert.pem",
        )
        .with_env_var(
            "xpack.security.transport.ssl.key",
            "certs/intermediate_server/private/elasticsearch-secure.key.pem",
        )
        .with_wait_for(WaitFor::StdOutMessage {
            message: "Active license is now [BASIC]; Security is enabled".to_string(),
        });

    let cli = testcontainers::clients::Cli::default();
    let container = cli.run(image);
    let host_port = container.get_host_port_ipv4(9200);

    let endpoint = format!("https://localhost:{}", host_port);

    run_insert_tests(
        ElasticsearchConfig {
            auth: Some(ElasticsearchAuth::Basic {
                user: "elastic".into(),
                password: "password".into(),
            }),
            endpoint,
            doc_type: Some("log_lines".into()),
            compression: Compression::None,
            tls: Some(TlsConfig {
                ca_file: Some(format!("{}/tests/ca/certs/ca.cert.pem", pwd).into()),
                verify_hostname: Some(false),
                ..Default::default()
            }),
            ..config()
        },
        false,
        BatchStatus::Delivered,
    )
    .await
}

async fn create_template_index(common: &ElasticsearchCommon, name: &str) -> crate::Result<()> {
    let mut builder = Request::put(format!("{}/_index_template/{}", common.base_url, name));
    if let Some(ce) = common.request_builder.compression.content_encoding() {
        builder = builder.header("Content-Encoding", ce);
    }

    for (k, v) in &common.request.headers {
        builder = builder.header(&k[..], &v[..]);
    }

    if let Some(auth) = &common.http_auth {
        builder = auth.apply_builder(builder);
    }

    builder = builder.header("Content-Type", "application/json");
    let req = builder
        .body(Bytes::from(
            r#"{"index_patterns":["*-*"],"data_stream":{}}"#,
        ))
        .unwrap();
    let proxy = ProxyConfig::default();
    let client = HttpClient::new(common.tls_settings.clone(), &proxy).unwrap();

    let resp = client
        .send(req.map(hyper::Body::from))
        .await
        .expect("Create template index failed");

    assert!(resp.is_successful());
    Ok(())
}

async fn create_data_stream(common: &ElasticsearchCommon, name: &str) -> crate::Result<()> {
    let mut builder = Request::put(format!("{}/_data_stream/{}", common.base_url, name));
    if let Some(ce) = common.request_builder.compression.content_encoding() {
        builder = builder.header("Content-Encoding", ce);
    }

    for (k, v) in &common.request.headers {
        builder = builder.header(&k[..], &v[..]);
    }

    if let Some(auth) = &common.http_auth {
        builder = auth.apply_builder(builder);
    }

    builder = builder.header("Content-Type", "application/json");
    let req = builder.body(Bytes::from("")).unwrap();
    let proxy = ProxyConfig::default();
    let client = HttpClient::new(common.tls_settings.clone(), &proxy).unwrap();

    let resp = client
        .send(req.map(hyper::Body::from))
        .await
        .expect("Create template index failed");
    assert!(resp.is_successful());

    Ok(())
}

// This implementation is ugly, cause async closure with argument is not stable yet. So
// we have to force the `sync` closure "f" return a future and then execute it.
fn setup_and_run<F, Fut>(f: F)
where
    F: FnOnce(String) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
{
    trace_init();

    let tag = "7.17.5";
    let image = GenericImage::new("elasticsearch", tag)
        .with_env_var("discovery.type", "single-node")
        .with_env_var("ingest.geoip.downloader.enabled", "false")
        .with_env_var("ES_JAVA_OPTS", "-Xms512m -Xmx512m")
        .with_wait_for(WaitFor::StdOutMessage {
            message: "Cluster health status changed from [YELLOW] to [GREEN]".to_string(),
        });

    let cli = testcontainers::clients::Cli::default();
    let container = cli.run(image);
    let host_port = container.get_host_port_ipv4(9200);

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Build async runtime failed");
    rt.block_on(f(format!("http://localhost:{}", host_port)));
}

#[test]
fn insert_events_in_data_stream() {
    setup_and_run(|endpoint| async move {
        let template_index = format!("template-{}", gen_index());
        let stream_index = format!("stream-{}", gen_index());

        let config = ElasticsearchConfig {
            endpoint,
            mode: ElasticsearchMode::DataStream,
            bulk: Some(BulkConfig {
                index: Some(stream_index.clone()),
                action: None,
            }),
            ..config()
        };
        let common = ElasticsearchCommon::parse_config(&config)
            .await
            .expect("Config error");

        create_template_index(&common, &template_index)
            .await
            .expect("Template index creation error");
        create_data_stream(&common, &stream_index)
            .await
            .expect("Data stream creation error");

        run_insert_tests_with_config(&config, false, BatchStatus::Delivered).await;
    });
}

#[test]
fn insert_events_with_failure() {
    setup_and_run(|endpoint| async move {
        run_insert_tests(
            ElasticsearchConfig {
                endpoint,
                doc_type: Some("log_lines".into()),
                compression: Compression::None,
                ..config()
            },
            true,
            BatchStatus::Rejected,
        )
        .await
    })
}
