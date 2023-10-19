use std::collections::BTreeMap;

use chrono::{TimeZone, Utc};
use codecs::encoding::Transformer;
use event::log::{parse_value_path, Value};
use event::{fields, LogRecord};
use framework::sink::util::Encoder;
use framework::template::Template;

use super::common::ElasticsearchCommon;
use super::config::{BulkConfig, Config};
use super::config::{DataStreamConfig, ElasticsearchMode};
use super::sink::process_log;
use super::BulkAction;

#[tokio::test]
async fn sets_create_action_when_configured() {
    let config = Config {
        bulk: Some(BulkConfig {
            action: Some(String::from("{{ action }}te")),
            index: Some(String::from("vertex")),
        }),
        endpoint: String::from("https://example.com"),
        ..Default::default()
    };
    let es = ElasticsearchCommon::parse_config(&config).await.unwrap();
    let log = LogRecord::from(fields!(
        "message" => "hi there",
        "timestamp" => Utc.with_ymd_and_hms(2020, 12, 1, 1, 2, 3).unwrap(),
        "action" => "crea"
    ));

    let mut encoded = vec![];
    let encode_size = es
        .request_builder
        .encoder
        .encode(
            vec![process_log(log, &es.mode, &None).unwrap()],
            &mut encoded,
        )
        .unwrap();

    let expected = r#"{"create":{"_index":"vertex","_type":"_doc"}}
{"action":"crea","message":"hi there","timestamp":"2020-12-01T01:02:03Z"}
"#;
    assert_eq!(std::str::from_utf8(&encoded).unwrap(), expected);
    assert_eq!(encoded.len(), encode_size);
}

fn data_stream_body() -> BTreeMap<String, Value> {
    fields!(
        "type" => "synthetics",
        "dataset" => "testing"
    )
}

#[tokio::test]
async fn encode_datastream_mode() {
    let config = Config {
        bulk: Some(BulkConfig {
            action: None,
            index: Some("index".into()),
        }),
        endpoint: "https://example.com".into(),
        mode: ElasticsearchMode::DataStream,
        ..Default::default()
    };
    let es = ElasticsearchCommon::parse_config(&config).await.unwrap();

    let log = LogRecord::from(fields!(
        "message" => "hi there",
        "timestamp" => Utc.with_ymd_and_hms(2020, 12, 1, 1, 2, 3).unwrap(),
        "data_stream" => data_stream_body()
    ));

    let mut encoded = vec![];
    let encoded_size = es
        .request_builder
        .encoder
        .encode(
            vec![process_log(log, &es.mode, &None).unwrap()],
            &mut encoded,
        )
        .unwrap();

    let expected = r#"{"create":{"_index":"synthetics-testing-default","_type":"_doc"}}
{"@timestamp":"2020-12-01T01:02:03Z","data_stream":{"dataset":"testing","namespace":"default","type":"synthetics"},"message":"hi there"}
"#;
    assert_eq!(std::str::from_utf8(&encoded).unwrap(), expected);
    assert_eq!(encoded.len(), encoded_size);
}

#[tokio::test]
async fn encode_datastream_mode_no_routing() {
    let config = Config {
        bulk: Some(BulkConfig {
            action: None,
            index: Some("index".into()),
        }),
        endpoint: "https://example.com".into(),
        mode: ElasticsearchMode::DataStream,
        data_stream: Some(DataStreamConfig {
            auto_routing: false,
            namespace: Template::try_from("something").unwrap(),
            ..Default::default()
        }),
        ..Default::default()
    };
    let es = ElasticsearchCommon::parse_config(&config).await.unwrap();
    let log = LogRecord::from(fields!(
        "message" => "hi there",
        "timestamp" => Utc.with_ymd_and_hms(2020, 12, 1, 1, 2, 3).unwrap(),
        "data_stream" => data_stream_body()
    ));

    let mut encoded = vec![];
    let encoded_size = es
        .request_builder
        .encoder
        .encode(
            vec![process_log(log, &es.mode, &None).unwrap()],
            &mut encoded,
        )
        .unwrap();

    let expected = r#"{"create":{"_index":"logs-generic-something","_type":"_doc"}}
{"@timestamp":"2020-12-01T01:02:03Z","data_stream":{"dataset":"testing","namespace":"something","type":"synthetics"},"message":"hi there"}
"#;
    assert_eq!(std::str::from_utf8(&encoded).unwrap(), expected);
    assert_eq!(encoded.len(), encoded_size);
}

#[tokio::test]
async fn decode_bulk_action_error() {
    let config = Config {
        bulk: Some(BulkConfig {
            action: Some("{{ action }}".into()),
            index: Some("index".into()),
        }),
        endpoint: "https://example.com".into(),
        ..Default::default()
    };
    let es = ElasticsearchCommon::parse_config(&config).await.unwrap();

    let log = LogRecord::from(fields!(
        "message" => "hi there",
        "foo" => "bar",
        "idx" => "purple",
    ));
    let action = es.mode.bulk_action(&log);
    assert!(action.is_none());
}

#[tokio::test]
async fn decode_bulk_action() {
    let config = Config {
        bulk: Some(BulkConfig {
            action: Some("create".into()),
            index: Some("index".into()),
        }),
        endpoint: "https://example.com".into(),
        ..Default::default()
    };
    let es = ElasticsearchCommon::parse_config(&config).await.unwrap();
    let log = LogRecord::from(fields!(
        "message" => "hi there",
    ));
    let action = es.mode.bulk_action(&log).unwrap();
    assert!(matches!(action, BulkAction::Create))
}

#[tokio::test]
async fn encode_datastream_mode_no_sync() {
    let config = Config {
        bulk: Some(BulkConfig {
            action: None,
            index: Some("index".into()),
        }),
        endpoint: "https://example.com".into(),
        mode: ElasticsearchMode::DataStream,
        data_stream: Some(DataStreamConfig {
            namespace: Template::try_from("something").unwrap(),
            sync_fields: false,
            ..Default::default()
        }),
        ..Default::default()
    };

    let es = ElasticsearchCommon::parse_config(&config).await.unwrap();
    let log = LogRecord::from(fields!(
        "message" => "hi there",
        "timestamp" => Utc.with_ymd_and_hms(2020, 12, 1, 1, 2, 3).unwrap(),
        "data_stream" => data_stream_body(),
    ));

    let mut encoded = vec![];
    let encoded_size = es
        .request_builder
        .encoder
        .encode(
            vec![process_log(log, &es.mode, &None).unwrap()],
            &mut encoded,
        )
        .unwrap();

    let expected = r#"{"create":{"_index":"synthetics-testing-something","_type":"_doc"}}
{"@timestamp":"2020-12-01T01:02:03Z","data_stream":{"dataset":"testing","type":"synthetics"},"message":"hi there"}
"#;
    assert_eq!(std::str::from_utf8(&encoded).unwrap(), expected);
    assert_eq!(encoded.len(), encoded_size);
}

#[tokio::test]
async fn allow_using_excepted_fields() {
    let config = Config {
        bulk: Some(BulkConfig {
            action: None,
            index: Some("{{ idx }}".into()),
        }),
        encoding: Transformer::new(
            None,
            Some(vec![
                parse_value_path("idx").unwrap(),
                parse_value_path("timestamp").unwrap(),
            ]),
            None,
        )
        .unwrap(),
        endpoint: "https://example.com".into(),
        ..Default::default()
    };
    let es = ElasticsearchCommon::parse_config(&config).await.unwrap();
    let log = LogRecord::from(fields!(
        "message" => "hi there",
        "foo" => "bar",
        "idx" => "purple"
    ));

    let mut encoded = vec![];
    let encoded_size = es
        .request_builder
        .encoder
        .encode(
            vec![process_log(log, &es.mode, &None).unwrap()],
            &mut encoded,
        )
        .unwrap();

    let expected = r#"{"index":{"_index":"purple","_type":"_doc"}}
{"foo":"bar","message":"hi there"}
"#;
    assert_eq!(std::str::from_utf8(&encoded).unwrap(), expected);
    assert_eq!(encoded.len(), encoded_size);
}
