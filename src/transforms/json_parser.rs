use configurable::configurable_component;
use event::log::OwnedTargetPath;
use event::Events;
use framework::config::{default_true, DataType, Output, TransformConfig, TransformContext};
use framework::{FunctionTransform, OutputBuffer, Transform};
use log_schema::log_schema;
use metrics::Counter;
use serde_json::Value;

#[configurable_component(transform, name = "json_parser")]
#[derive(Clone)]
#[serde(deny_unknown_fields, default)]
pub struct Config {
    /// Which field to parse, by default log_schema's message key is used.
    pub field: Option<OwnedTargetPath>,

    /// Should Vertex drop the invalid event.
    #[serde(default = "default_true")]
    pub drop_invalid: bool,

    #[serde(default = "default_true")]
    pub drop_field: bool,

    /// Which field to store the parsed result. If this is not set, the resultd
    /// will set as log's fields, which means other filed will dropped.
    pub target_field: Option<OwnedTargetPath>,

    /// Overwrite target filed.
    #[serde(default)]
    pub overwrite_target: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            field: None,
            drop_invalid: false,
            drop_field: true,
            target_field: None,
            overwrite_target: false,
        }
    }
}

#[async_trait::async_trait]
#[typetag::serde(name = "json_parser")]
impl TransformConfig for Config {
    async fn build(&self, _cx: &TransformContext) -> crate::Result<Transform> {
        Ok(Transform::function(JsonParser::from(self.clone())))
    }

    fn input_type(&self) -> DataType {
        DataType::Log
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Log)]
    }

    fn enable_concurrency(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone)]
pub struct JsonParser {
    field: OwnedTargetPath,
    drop_invalid: bool,
    drop_field: bool,
    target_field: Option<OwnedTargetPath>,
    overwrite_target: bool,

    // metrics
    discarded_events: Counter,
    invalid_events: Counter,
}

impl From<Config> for JsonParser {
    fn from(config: Config) -> JsonParser {
        let field = config
            .field
            .unwrap_or_else(|| log_schema().message_key().clone());

        JsonParser {
            field,
            drop_invalid: config.drop_invalid,
            drop_field: config.drop_field,
            target_field: config.target_field,
            overwrite_target: config.overwrite_target,
            discarded_events: metrics::register_counter("component_errors_total", "").recorder(&[]),
            invalid_events: metrics::register_counter("component_errors_total", "")
                .recorder(&[("error", "invalid json")]),
        }
    }
}

impl FunctionTransform for JsonParser {
    fn transform(&mut self, output: &mut OutputBuffer, events: Events) {
        if let Events::Logs(logs) = events {
            for mut log in logs {
                let value = log.get(&self.field);

                let parsed = value
                    .and_then(|value| {
                        let to_parse = value.coerce_to_bytes();
                        serde_json::from_slice::<Value>(to_parse.as_ref())
                            .map_err(|err| {
                                warn!(
                                    message = "Event failed to parse as JSON",
                                    field = %self.field,
                                    ?value,
                                    ?err,
                                    internal_log_rate_limit = true
                                );

                                self.invalid_events.inc(1);
                                if self.drop_field {
                                    self.discarded_events.inc(1);
                                }
                            })
                            .ok()
                    })
                    .and_then(|value| {
                        if let Value::Object(object) = value {
                            Some(object)
                        } else {
                            None
                        }
                    });

                if let Some(object) = parsed {
                    match self.target_field {
                        Some(ref target_field) => {
                            let contains_target = log.contains(target_field);

                            if contains_target && !self.overwrite_target {
                                warn!(
                                    message = "Target field already exists",
                                    target = ?target_field,
                                    internal_log_rate_limit = true
                                );

                                // TODO: metrics
                            } else {
                                if self.drop_field {
                                    log.remove(&self.field);
                                }

                                log.insert(target_field, Value::Object(object));
                            }
                        }
                        None => {
                            if self.drop_field {
                                log.remove(&self.field);
                            }

                            if let event::log::Value::Object(map) = log.value_mut() {
                                for (key, value) in object {
                                    map.insert(key, value.into());
                                }
                            }
                        }
                    }
                } else if self.drop_invalid {
                    // TODO: metrics
                    continue;
                }

                output.push_one(log.into());
            }
        }
    }
}

#[cfg(test)]
mod test {
    use event::log::path::parse_target_path;
    use event::Event;
    use serde_json::json;

    use super::*;
    use crate::transforms::transform_one;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>();
    }

    #[test]
    fn json_parser_drop_field() {
        let mut parser = JsonParser::from(Config::default());

        let event = Event::from(r#"{"greeting": "hello", "name": "bob"}"#);
        let metadata = event.metadata().clone();

        let event = transform_one(&mut parser, event).unwrap();
        assert!(event.as_log().get(log_schema().message_key()).is_none());
        assert_eq!(event.metadata(), &metadata);
    }

    #[test]
    fn json_parser_doesnt_drop_field() {
        let mut parser = JsonParser::from(Config {
            drop_field: false,
            ..Default::default()
        });

        let event = Event::from(r#"{"greeting": "hello", "name": "bob"}"#);
        let metadata = event.metadata().clone();

        let event = transform_one(&mut parser, event).unwrap();
        assert!(event.as_log().get(log_schema().message_key()).is_some());
        assert_eq!(event.metadata(), &metadata);
    }

    #[test]
    fn json_parser_parse_raw() {
        let mut parser = JsonParser::from(Config {
            drop_field: false,
            ..Default::default()
        });

        let event = Event::from(r#"{"greeting": "hello", "name": "bob"}"#);
        let metadata = event.metadata().clone();

        let event = transform_one(&mut parser, event).unwrap();

        assert_eq!(*event.as_log().get("greeting").unwrap(), "hello".into());
        assert_eq!(*event.as_log().get("name").unwrap(), "bob".into());
        assert_eq!(
            *event.as_log().get(log_schema().message_key()).unwrap(),
            r#"{"greeting": "hello", "name": "bob"}"#.into()
        );
        assert_eq!(event.metadata(), &metadata);
    }

    // Ensure the JSON parser doesn't take strings as toml paths.
    // This is a regression test, see: https://github.com/timberio/vector/issues/2814
    #[test]
    fn json_parser_parse_periods() {
        let mut parser = JsonParser::from(Config {
            drop_field: false,
            ..Default::default()
        });

        let test_json = json!({
            "field.with.dots": "hello",
            "sub.field": { "another.one": "bob"},
        });

        let event = Event::from(test_json.to_string());
        let metadata = event.metadata().clone();

        let event = transform_one(&mut parser, event).unwrap();

        assert_eq!(
            event.as_log().get("\"field.with.dots\""),
            Some(&event::log::Value::from("hello")),
        );
        assert_eq!(
            event.as_log().get("\"sub.field\""),
            Some(&event::log::Value::from(json!({ "another.one": "bob", }))),
        );
        assert_eq!(event.metadata(), &metadata);
    }

    #[test]
    fn json_parser_parse_raw_with_whitespace() {
        let mut parser = JsonParser::from(Config {
            drop_field: false,
            ..Default::default()
        });

        let event = Event::from(r#" {"greeting": "hello", "name": "bob"}    "#);
        let metadata = event.metadata().clone();

        let event = transform_one(&mut parser, event).unwrap();

        assert_eq!(*event.as_log().get("greeting").unwrap(), "hello".into());
        assert_eq!(*event.as_log().get("name").unwrap(), "bob".into());
        assert_eq!(
            *event.as_log().get(log_schema().message_key()).unwrap(),
            r#" {"greeting": "hello", "name": "bob"}    "#.into()
        );
        assert_eq!(event.metadata(), &metadata);
    }

    #[test]
    fn json_parser_parse_field() {
        let mut parser = JsonParser::from(Config {
            field: Some(parse_target_path("data").unwrap()),
            drop_field: false,
            ..Default::default()
        });

        // Field present

        let mut event = Event::from("message");
        event
            .as_mut_log()
            .insert("data", r#"{"greeting": "hello", "name": "bob"}"#);
        let metadata = event.metadata().clone();

        let event = transform_one(&mut parser, event).unwrap();

        event.as_log().get("greeting").unwrap();

        assert_eq!(*event.as_log().get("greeting").unwrap(), "hello".into(),);
        assert_eq!(*event.as_log().get("name").unwrap(), "bob".into());
        assert_eq!(
            *event.as_log().get("data").unwrap(),
            r#"{"greeting": "hello", "name": "bob"}"#.into()
        );
        assert_eq!(event.metadata(), &metadata);

        // Field missing
        let event = Event::from("message");
        let metadata = event.metadata().clone();

        let parsed = transform_one(&mut parser, event.clone()).unwrap();

        assert_eq!(event, parsed);
        assert_eq!(event.metadata(), &metadata);
    }

    #[test]
    fn json_parser_parse_inner_json() {
        let mut parser_outer = JsonParser::from(Config {
            ..Default::default()
        });

        let mut parser_inner = JsonParser::from(Config {
            field: Some(parse_target_path("log").unwrap()),
            ..Default::default()
        });

        let event = Event::from(
            r#"{"log":"{\"type\":\"response\",\"@timestamp\":\"2018-10-04T21:12:33Z\",\"tags\":[],\"pid\":1,\"method\":\"post\",\"statusCode\":200,\"req\":{\"url\":\"/elasticsearch/_msearch\",\"method\":\"post\",\"headers\":{\"host\":\"logs.com\",\"connection\":\"close\",\"x-real-ip\":\"120.21.3.1\",\"x-forwarded-for\":\"121.91.2.2\",\"x-forwarded-host\":\"logs.com\",\"x-forwarded-port\":\"443\",\"x-forwarded-proto\":\"https\",\"x-original-uri\":\"/elasticsearch/_msearch\",\"x-scheme\":\"https\",\"content-length\":\"1026\",\"accept\":\"application/json, text/plain, */*\",\"origin\":\"https://logs.com\",\"kbn-version\":\"5.2.3\",\"user-agent\":\"Mozilla/5.0 (Macintosh; Intel Mac OS X 10_12_6) AppleWebKit/532.30 (KHTML, like Gecko) Chrome/62.0.3361.210 Safari/533.21\",\"content-type\":\"application/x-ndjson\",\"referer\":\"https://domain.com/app/kibana\",\"accept-encoding\":\"gzip, deflate, br\",\"accept-language\":\"en-US,en;q=0.8\"},\"remoteAddress\":\"122.211.22.11\",\"userAgent\":\"22.322.32.22\",\"referer\":\"https://domain.com/app/kibana\"},\"res\":{\"statusCode\":200,\"responseTime\":417,\"contentLength\":9},\"message\":\"POST /elasticsearch/_msearch 200 225ms - 8.0B\"}\n","stream":"stdout","time":"2018-10-02T21:14:48.2233245241Z"}"#,
        );
        let metadata = event.metadata().clone();

        let parsed_event = transform_one(&mut parser_outer, event).unwrap();

        assert_eq!(
            *parsed_event.as_log().get("stream").unwrap(),
            "stdout".into()
        );
        assert_eq!(parsed_event.metadata(), &metadata);

        let parsed_inner_event = transform_one(&mut parser_inner, parsed_event).unwrap();
        let log = parsed_inner_event.into_log();

        assert_eq!(*log.get("type").unwrap(), "response".into());
        assert_eq!(*log.get("statusCode").unwrap(), 200.into());
        assert_eq!(log.metadata(), &metadata);
    }

    #[test]
    fn json_parser_invalid_json() {
        let invalid = r#"{"greeting": "hello","#;

        // Raw
        let mut parser = JsonParser::from(Config {
            drop_field: false,
            ..Default::default()
        });

        let event = Event::from(invalid);
        let metadata = event.metadata().clone();

        let parsed = transform_one(&mut parser, event.clone()).unwrap();

        assert_eq!(event, parsed);
        assert_eq!(
            *event.as_log().get(log_schema().message_key()).unwrap(),
            invalid.into()
        );
        assert_eq!(event.metadata(), &metadata);

        // Field
        let mut parser = JsonParser::from(Config {
            field: Some(parse_target_path("data").unwrap()),
            drop_field: false,
            ..Default::default()
        });

        let mut event = Event::from("message");
        event.as_mut_log().insert("data", invalid);

        let event = transform_one(&mut parser, event).unwrap();

        assert_eq!(*event.as_log().get("data").unwrap(), invalid.into());
        assert!(event.as_log().get("greeting").is_none());
    }

    #[test]
    fn json_parser_drop_invalid() {
        let valid = r#"{"greeting": "hello", "name": "bob"}"#;
        let invalid = r#"{"greeting": "hello","#;
        let not_object = r#""hello""#;

        // Raw
        let mut parser = JsonParser::from(Config {
            drop_invalid: true,
            ..Default::default()
        });

        let event = Event::from(valid);
        assert!(transform_one(&mut parser, event).is_some());

        let event = Event::from(invalid);
        let n = transform_one(&mut parser, event);
        assert!(n.is_none());

        let event = Event::from(not_object);
        assert!(transform_one(&mut parser, event).is_none());

        // Field
        let mut parser = JsonParser::from(Config {
            field: Some(parse_target_path("data").unwrap()),
            drop_invalid: true,
            ..Default::default()
        });

        let mut event = Event::from("message");
        event.as_mut_log().insert("data", valid);
        assert!(transform_one(&mut parser, event).is_some());

        let mut event = Event::from("message");
        event.as_mut_log().insert("data", invalid);
        assert!(transform_one(&mut parser, event).is_none());

        let mut event = Event::from("message");
        event.as_mut_log().insert("data", not_object);
        assert!(transform_one(&mut parser, event).is_none());

        // Missing field
        let event = Event::from("message");
        assert!(transform_one(&mut parser, event).is_none());
    }

    #[test]
    fn json_parser_chained() {
        let mut parser1 = JsonParser::from(Config {
            ..Default::default()
        });
        let mut parser2 = JsonParser::from(Config {
            field: Some(parse_target_path("nested").unwrap()),
            ..Default::default()
        });

        let event = Event::from(
            r#"{"greeting": "hello", "name": "bob", "nested": "{\"message\": \"help i'm trapped under many layers of json\"}"}"#,
        );
        let metadata = event.metadata().clone();
        let event = transform_one(&mut parser1, event).unwrap();
        let event = transform_one(&mut parser2, event).unwrap();

        assert_eq!(*event.as_log().get("greeting").unwrap(), "hello".into());
        assert_eq!(*event.as_log().get("name").unwrap(), "bob".into());
        assert_eq!(
            *event.as_log().get("message").unwrap(),
            "help i'm trapped under many layers of json".into()
        );
        assert_eq!(event.metadata(), &metadata);
    }

    #[test]
    fn json_parser_types() {
        let mut parser = JsonParser::from(Config {
            ..Default::default()
        });

        let event = Event::from(
            r#"{
              "string": "this is text",
              "null": null,
              "float": 12.34,
              "int": 56,
              "bool true": true,
              "bool false": false,
              "array": ["z", 7],
              "object": { "nested": "data", "more": "values" },
              "deep": [[[{"a": { "b": { "c": [[[1234]]]}}}]]]
            }"#,
        );
        let metadata = event.metadata().clone();
        let event = transform_one(&mut parser, event).unwrap();

        assert_eq!(
            *event.as_log().get("string").unwrap(),
            "this is text".into()
        );
        assert_eq!(
            *event.as_log().get("null").unwrap(),
            event::log::Value::Null
        );
        assert_eq!(*event.as_log().get("float").unwrap(), 12.34.into());
        assert_eq!(*event.as_log().get("int").unwrap(), 56.into());
        assert_eq!(*event.as_log().get("\"bool true\"").unwrap(), true.into());
        assert_eq!(*event.as_log().get("\"bool false\"").unwrap(), false.into());
        assert_eq!(*event.as_log().get("array[0]").unwrap(), "z".into());
        assert_eq!(*event.as_log().get("array[1]").unwrap(), 7.into());
        assert_eq!(*event.as_log().get("object.nested").unwrap(), "data".into());
        assert_eq!(*event.as_log().get("object.more").unwrap(), "values".into());
        assert_eq!(
            *event.as_log().get("deep[0][0][0].a.b.c[0][0][0]").unwrap(),
            1234.into()
        );
        assert_eq!(event.metadata(), &metadata);
    }

    #[test]
    fn drop_field_before_adding() {
        let mut parser = JsonParser::from(Config {
            drop_field: true,
            ..Default::default()
        });

        let event = Event::from(
            r#"{
                "key": "data",
                "message": "inner"
            }"#,
        );
        let metadata = event.metadata().clone();

        let event = transform_one(&mut parser, event).unwrap();

        assert_eq!(*event.as_log().get("key").unwrap(), "data".into());
        assert_eq!(*event.as_log().get("message").unwrap(), "inner".into());
        assert_eq!(event.metadata(), &metadata);
    }

    #[test]
    fn doesnt_drop_field_after_failed_parse() {
        let mut parser = JsonParser::from(Config {
            drop_field: true,
            ..Default::default()
        });

        let event = Event::from(r#"invalid json"#);
        let metadata = event.metadata().clone();

        let event = transform_one(&mut parser, event).unwrap();

        assert_eq!(
            *event.as_log().get("message").unwrap(),
            "invalid json".into()
        );
        assert_eq!(event.metadata(), &metadata);
    }

    #[test]
    fn target_field_works() {
        let mut parser = JsonParser::from(Config {
            drop_field: false,
            target_field: Some(parse_target_path("that").unwrap()),
            ..Default::default()
        });

        let event = Event::from(r#"{"greeting": "hello", "name": "bob"}"#);
        let metadata = event.metadata().clone();
        let event = transform_one(&mut parser, event).unwrap();
        let event = event.as_log();

        assert_eq!(*event.get("that.greeting").unwrap(), "hello".into());
        assert_eq!(*event.get("that.name").unwrap(), "bob".into());
        assert_eq!(event.metadata(), &metadata);
    }

    #[test]
    fn target_field_preserves_existing() {
        let mut parser = JsonParser::from(Config {
            drop_field: false,
            target_field: Some(parse_target_path("message").unwrap()),
            ..Default::default()
        });

        let message = r#"{"greeting": "hello", "name": "bob"}"#;
        let event = Event::from(message);
        let metadata = event.metadata().clone();
        let event = transform_one(&mut parser, event).unwrap();
        let event = event.as_log();

        assert_eq!(*event.get("message").unwrap(), message.into());
        assert_eq!(event.get("message.greeting"), None);
        assert_eq!(event.get("message.name"), None);
        assert_eq!(event.metadata(), &metadata);
    }

    #[test]
    fn target_field_overwrites_existing() {
        let mut parser = JsonParser::from(Config {
            drop_field: false,
            target_field: Some(parse_target_path("message").unwrap()),
            overwrite_target: true,
            ..Default::default()
        });

        let message = r#"{"greeting": "hello", "name": "bob"}"#;
        let event = Event::from(message);
        let metadata = event.metadata().clone();
        let event = transform_one(&mut parser, event).unwrap();
        let event = event.as_log();

        match event.get("message") {
            Some(event::log::Value::Object(_)) => (),
            _ => panic!("\"message\" is not a map"),
        }
        assert_eq!(*event.get("message.greeting").unwrap(), "hello".into());
        assert_eq!(*event.get("message.name").unwrap(), "bob".into());
        assert_eq!(event.metadata(), &metadata);
    }
}
