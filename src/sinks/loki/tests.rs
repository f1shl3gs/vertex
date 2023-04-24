use event::{fields, Event};
use framework::config::ProxyConfig;
use framework::http::HttpClient;
use framework::sink::util::testing::{build_test_server, load_sink};
use futures_util::StreamExt;

use super::config::LokiConfig;
use super::healthcheck::health_check;
use super::sink::LokiSink;

#[test]
fn interpolate_labels() {
    let (config, cx) = load_sink::<LokiConfig>(
        r#"
endpoint: http://localhost:3100
labels:
    label1: "{{ foo }}"
    label2: some-static-label
    label3: "{{ foo }}"
    "{{ foo }}": "{{ foo }}"
encoding:
    codec: json
remove_label_fields: true
"#,
    )
    .unwrap();
    let client = config.build_client(cx.clone()).unwrap();
    let mut sink = LokiSink::new(config, client, cx).unwrap();

    let event1 = Event::from(fields!(
        "message" => "hello",
        "foo" => "bar"
    ));

    let mut record = sink.encoder.encode_event(event1);

    // HashMap -> Vec doesn't like keeping ordering
    record.labels.sort();

    // The final event should have timestamps and labels removed
    let expected = serde_json::to_string(&serde_json::json!({
            "message": "hello"
    }))
    .unwrap();

    assert_eq!(record.event.event, expected);
    assert_eq!(record.labels[0], ("bar".to_string(), "bar".to_string()));
    assert_eq!(record.labels[1], ("label1".to_string(), "bar".to_string()));
    assert_eq!(
        record.labels[2],
        ("label2".to_string(), "some-static-label".to_string())
    );

    // make sure we can reuse fields across labels
    assert_eq!(record.labels[3], ("label3".to_string(), "bar".to_string()))
}

#[test]
fn use_label_from_dropped_fields() {
    let (config, cx) = load_sink::<LokiConfig>(
        r#"
endpoint: http://localhost:3100
labels:
    bar: "{{ foo }}"
encoding:
    codec: json
    except_fields:
        - foo
"#,
    )
    .unwrap();

    let client = config.build_client(cx.clone()).unwrap();
    let mut sink = LokiSink::new(config, client, cx).unwrap();

    let mut event = Event::from("hello");
    event.as_mut_log().insert_field("foo", "bar");
    let record = sink.encoder.encode_event(event);
    let want = serde_json::to_string(&serde_json::json!({
            "message": "hello",
    }))
    .unwrap();

    assert_eq!(record.event.event, want);
    assert_eq!(record.labels[0], ("bar".to_string(), "bar".to_string()));
}

#[tokio::test]
async fn health_check_includes_auth() {
    let (mut config, _cx) = load_sink::<LokiConfig>(
        r#"
endpoint: http://localhost:3100
labels:
    test_name: placeholder
encoding:
    codec: json
auth:
    strategy: basic
    user: username
    password: some_password
"#,
    )
    .unwrap();

    let addr = testify::next_addr();
    let endpoint = format!("http://{}", addr);
    config.endpoint = endpoint
        .clone()
        .parse::<http::Uri>()
        .expect("Could not create URI")
        .into();

    let (rx, _trigger, server) = build_test_server(addr);
    tokio::spawn(server);

    let proxy = ProxyConfig::default();
    let client = HttpClient::new(&config.tls, &proxy).expect("Could not create http client");

    health_check(config, client)
        .await
        .expect("health check failed");

    let output = rx.take(1).collect::<Vec<_>>().await;

    assert_eq!(
        Some(&http::header::HeaderValue::from_static(
            "Basic dXNlcm5hbWU6c29tZV9wYXNzd29yZA=="
        )),
        output[0].0.headers.get("authorization")
    )
}
