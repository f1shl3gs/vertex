use chrono::Utc;
use event::{fields, LogRecord};
use framework::sink::util::Encoder;

use super::common::ElasticsearchCommon;
use super::config::{BulkConfig, ElasticsearchConfig};
use super::sink::process_log;

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
    let mut log = LogRecord::from(fields!(
        log_schema().message_key() => "hi there",
        log_schema().timestamp_key() => Utc.ymd(2020, 12, 1).and_hms(1, 2, 3),
        "action", "crea"
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

    let expected = r#"{"create":{"_index":"vector","_type":"_doc"}}
{"action":"crea","message":"hi there","timestamp":"2020-12-01T01:02:03Z"}
"#;
    assert_eq!(std::str::from_utf8(&encoded).unwrap(), expected);
    assert_eq!(encoded.len(), encode_size);
}
