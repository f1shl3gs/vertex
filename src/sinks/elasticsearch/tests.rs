use std::collections::BTreeMap;

use chrono::{TimeZone, Utc};
use event::log::Value;
use event::{fields, LogRecord};
use framework::sink::util::{Encoder, Transformer};
use framework::template::Template;
use log_schema::log_schema;

use super::common::ElasticsearchCommon;
use super::config::{BulkConfig, ElasticsearchConfig};
use super::config::{DataStreamConfig, ElasticsearchMode};
use super::sink::process_log;
use super::BulkAction;

#[tokio::test]
async fn sets_create_action_when_configured() {
    let config = ElasticsearchConfig {
        bulk: Some(BulkConfig {
            action: Some(String::from("{{ action }}te")),
            index: Some(String::from("vertex")),
        }),
        endpoint: String::from("https://example.com"),
        ..Default::default()
    };
    let es = ElasticsearchCommon::parse_config(&config).await.unwrap();
    let log = LogRecord::from(fields!(
        log_schema().message_key() => "hi there",
        log_schema().timestamp_key() => Utc.ymd(2020, 12, 1).and_hms(1, 2, 3),
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
    let config = ElasticsearchConfig {
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
        log_schema().message_key() => "hi there",
        log_schema().timestamp_key() => Utc.ymd(2020, 12, 1).and_hms(1, 2, 3),
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
    let config = ElasticsearchConfig {
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
        log_schema().message_key() => "hi there",
        log_schema().timestamp_key() => Utc.ymd(2020, 12, 1).and_hms(1, 2, 3),
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
    let config = ElasticsearchConfig {
        bulk: Some(BulkConfig {
            action: Some("{{ action }}".into()),
            index: Some("index".into()),
        }),
        endpoint: "https://example.com".into(),
        ..Default::default()
    };
    let es = ElasticsearchCommon::parse_config(&config).await.unwrap();

    let log = LogRecord::from(fields!(
        log_schema().message_key() => "hi there",
        "foo" => "bar",
        "idx" => "purple",
    ));
    let action = es.mode.bulk_action(&log);
    assert!(action.is_none());
}

#[tokio::test]
async fn decode_bulk_action() {
    let config = ElasticsearchConfig {
        bulk: Some(BulkConfig {
            action: Some("create".into()),
            index: Some("index".into()),
        }),
        endpoint: "https://example.com".into(),
        ..Default::default()
    };
    let es = ElasticsearchCommon::parse_config(&config).await.unwrap();
    let log = LogRecord::from(fields!(
        log_schema().message_key() => "hi there",
    ));
    let action = es.mode.bulk_action(&log).unwrap();
    assert!(matches!(action, BulkAction::Create))
}

#[tokio::test]
async fn encode_datastream_mode_no_sync() {
    let config = ElasticsearchConfig {
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
        log_schema().message_key() => "hi there",
        log_schema().timestamp_key() => Utc.ymd(2020, 12, 1).and_hms(1, 2, 3),
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
    let config = ElasticsearchConfig {
        bulk: Some(BulkConfig {
            action: None,
            index: Some("{{ idx }}".into()),
        }),
        encoding: Transformer::new(
            None,
            Some(vec!["idx".to_string(), "timestamp".to_string()]),
            None,
        )
        .unwrap(),
        endpoint: "https://example.com".into(),
        ..Default::default()
    };
    let es = ElasticsearchCommon::parse_config(&config).await.unwrap();
    let log = LogRecord::from(fields!(
        log_schema().message_key() => "hi there",
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
