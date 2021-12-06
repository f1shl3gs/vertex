use event::Event;
use crate::sinks::loki::config::LokiConfig;
use crate::sinks::loki::sink::LokiSink;
use crate::sinks::util::testing::load_sink;

#[test]
fn interpolate_labels() {
    let (config, cx) = load_sink::<LokiConfig>(r#"
endpoint: http://localhost:3100
labels: { label1 = "{{ foo }}", label2 = "some-static-label", label3 = "{{ foo }}", "{{ foo }}" = "{{ foo }}" }
encoding: json
remove_label_fields: true
"#).unwrap();
    let client = config.build_client(cx.clone()).unwrap();
    let sink = LokiSink::new(config, client, cx).unwrap();

    let mut event1 = Event::from("hello");
    event1.as_mut_log()
        .insert_field("foo", "bar");

    let mut record = sink.encoder.encode_event(event1);

    // HashMap -> Vec doesn't like keeping ordering
    record.labels.sort();

    // The final event should have timestamps and labels removed
    let expected = serde_json::to_string(&serde_json::json!({
        "message": "hello"
    })).unwrap();

    assert_eq!(record.event.event, expected);
    assert_eq!(record.labels[0], ("bar".to_string(), "bar".to_string()));
    assert_eq!(record.labels[1], ("label1".to_string(), "bar".to_string()));
    assert_eq!(record.labels[2], ("label2".to_string(), "some-static-label".to_string()));

    // make sure we can reuse fields across labels
    assert_eq!(record.labels[3], ("label3".to_string(), "bar".to_string()))
}