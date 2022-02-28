mod codec;
mod config;

use std::io;
use std::io::Write;
use std::sync::Arc;

use event::log::path_iter::{PathComponent, PathIter};
use event::{Event, LogRecord, MaybeAsLogMut, Value};
use serde::{Deserialize, Serialize};

// re-export
pub use codec::*;
pub use config::*;

/// You'll find three encoding configuration types that can be used
///     * [`EncodingConfig<E>`]
///     * [`EncodingConfigWithDefault<E>`]
///     * [`EncodingConfigFixed<E>`]
///

pub trait Encoder<T> {
    /// Encodes the input into the provided writer
    ///
    /// # Errors
    ///
    /// If an I/O error is encountered while encoding the input, an error variant will
    /// be returned.
    fn encode(&self, input: T, writer: &mut dyn io::Write) -> io::Result<usize>;
}

impl<E, T> Encoder<T> for Arc<E>
where
    E: Encoder<T>,
{
    fn encode(&self, input: T, writer: &mut dyn Write) -> io::Result<usize> {
        (**self).encode(input, writer)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TimestampFormat {
    Unix,
    RFC3339,
}

/// The behavior of a encoding configuration
pub trait EncodingConfiguration {
    type Codec;
    // Required Accessors

    fn codec(&self) -> &Self::Codec;
    fn schema(&self) -> &Option<String>;
    fn only_fields(&self) -> &Option<Vec<Vec<PathComponent>>>;
    fn except_fields(&self) -> &Option<Vec<String>>;
    fn timestamp_format(&self) -> &Option<TimestampFormat>;

    fn apply_only_fields(&self, log: &mut LogRecord) {
        if let Some(only_fields) = &self.only_fields() {
            let mut to_remove = log
                .keys()
                .filter(|field| {
                    let field_path = PathIter::new(field).collect::<Vec<_>>();
                    !only_fields
                        .iter()
                        .any(|only| field_path.starts_with(&only[..]))
                })
                .collect::<Vec<_>>();

            // reverse sort so that we delete array elements at the end first rather than the start
            // so that any `nulls` at the end are dropped and empty arrays are pruned
            to_remove.sort_by(|a, b| b.cmp(a));

            for removal in to_remove {
                log.remove_field_prune(removal, true);
            }
        }
    }

    fn apply_except_fields(&self, log: &mut LogRecord) {
        if let Some(except_fields) = &self.except_fields() {
            for field in except_fields {
                log.remove_field(field);
            }
        }
    }

    fn apply_timestamp_format(&self, log: &mut LogRecord) {
        if let Some(format) = &self.timestamp_format() {
            match format {
                TimestampFormat::Unix => {
                    let mut timestamps = Vec::new();
                    for (k, v) in log.all_fields() {
                        if let Value::Timestamp(ts) = v {
                            timestamps.push((k.clone(), Value::Int64(ts.timestamp())))
                        }
                    }

                    for (k, v) in timestamps {
                        log.insert_field(k, v);
                    }
                }

                // RFC3339 is the default serialization of a timestamp
                TimestampFormat::RFC3339 => (),
            }
        }
    }

    /// Check that the configuration is valid.
    ///
    /// If an error is returned, the entire encoding configuration should be considered inoperable.
    ///
    /// For example, this checks if `except_fields` and `only_fields` items are mutually exclusive.
    fn validate(&self) -> Result<(), std::io::Error> {
        if let (Some(only_fields), Some(expect_fields)) =
            (&self.only_fields(), &self.except_fields())
        {
            if expect_fields.iter().any(|f| {
                let path_iter = PathIter::new(f).collect::<Vec<_>>();
                only_fields.iter().any(|v| v == &path_iter)
            }) {
                let err = std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "`expect_fields` and `only_fields` should be mutually exclusive",
                );

                return Err(err);
            }
        }

        Ok(())
    }

    /// Apply the EncodingConfig rules to the provided event.
    ///
    /// Currently, this is idempotent.
    fn apply_rules<T>(&self, event: &mut T)
    where
        T: MaybeAsLogMut,
    {
        // No rules are currently applied to metrics
        if let Some(log) = event.maybe_as_log_mut() {
            // Ordering in here should not matter
            self.apply_except_fields(log);
            self.apply_only_fields(log);
            self.apply_timestamp_format(log);
        }
    }
}

pub trait VisitLogMut {
    fn visit_logs_mut<F>(&mut self, func: F)
    where
        F: Fn(&mut LogRecord);
}

impl<T> VisitLogMut for Vec<T>
where
    T: VisitLogMut,
{
    fn visit_logs_mut<F>(&mut self, func: F)
    where
        F: Fn(&mut LogRecord),
    {
        for item in self {
            item.visit_logs_mut(&func);
        }
    }
}

impl VisitLogMut for Event {
    fn visit_logs_mut<F>(&mut self, func: F)
    where
        F: Fn(&mut LogRecord),
    {
        if let Event::Log(log) = self {
            func(log)
        }
    }
}

impl VisitLogMut for LogRecord {
    fn visit_logs_mut<F>(&mut self, func: F)
    where
        F: Fn(&mut LogRecord),
    {
        func(self)
    }
}

impl<E, T> Encoder<T> for E
where
    E: EncodingConfiguration,
    E::Codec: Encoder<T>,
    T: VisitLogMut,
{
    fn encode(&self, mut input: T, writer: &mut dyn Write) -> io::Result<usize> {
        input.visit_logs_mut(|log| self.apply_rules(log));
        self.codec().encode(input, writer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use event::Value;
    use indoc::indoc;
    use log_schema::log_schema;
    use std::collections::BTreeMap;

    #[derive(Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
    enum TestEncoding {
        Snoot,
        Boop,
    }

    #[derive(Deserialize, Serialize, Debug)]
    #[serde(deny_unknown_fields)]
    struct TestConfig {
        encoding: EncodingConfig<TestEncoding>,
    }

    // TODO(2410): Using PathComponents here is a hack for #2407, #2410 should fix this fully.
    fn as_path_components(a: &str) -> Vec<PathComponent> {
        PathIter::new(a).collect()
    }

    const YAML_SIMPLE_STRING: &str = r#"encoding: "Snoot""#;

    #[test]
    fn config_string() {
        let config: TestConfig = serde_yaml::from_str(YAML_SIMPLE_STRING).unwrap();
        config.encoding.validate().unwrap();
        assert_eq!(config.encoding.codec(), &TestEncoding::Snoot);
    }

    const YAML_SIMPLE_STRUCT: &str = indoc! {r#"
        encoding:
            codec: "Snoot"
            except_fields: ["Doop"]
            only_fields: ["Boop"]
    "#};

    #[test]
    fn config_struct() {
        let config: TestConfig = serde_yaml::from_str(YAML_SIMPLE_STRUCT).unwrap();
        config.encoding.validate().unwrap();
        assert_eq!(config.encoding.codec, TestEncoding::Snoot);
        assert_eq!(config.encoding.except_fields, Some(vec!["Doop".into()]));
        assert_eq!(
            config.encoding.only_fields,
            Some(vec![as_path_components("Boop")])
        );
    }

    const YAML_EXCLUSIVITY_VIOLATION: &str = indoc! {r#"
        encoding:
            codec: "Snoot"
            except_fields: ["Doop"]
            only_fields: ["Doop"]
    "#};

    #[test]
    fn exclusivity_violation() {
        let config: std::result::Result<TestConfig, _> =
            serde_yaml::from_str(YAML_EXCLUSIVITY_VIOLATION);
        assert!(config.is_err())
    }

    const YAML_EXCEPT_FIELD: &str = indoc! {r#"
        encoding:
            codec: "Snoot"
            except_fields:
                - "a.b.c"
                - "b"
                - "c[0].y"
                - "d\\.z"
                - "e"
    "#};

    #[test]
    fn test_except() {
        let config: TestConfig = serde_yaml::from_str(YAML_EXCEPT_FIELD).unwrap();
        config.encoding.validate().unwrap();
        let mut event = Event::new_empty_log();
        {
            let log = event.as_mut_log();
            log.insert_field("a", 1);
            log.insert_field("a.b", 1);
            log.insert_field("a.b.c", 1);
            log.insert_field("a.b.d", 1);
            log.insert_field("b[0]", 1);
            log.insert_field("b[1].x", 1);
            log.insert_field("c[0].x", 1);
            log.insert_field("c[0].y", 1);
            log.insert_field("d\\.z", 1);
            log.insert_field("e.a", 1);
            log.insert_field("e.b", 1);
        }
        config.encoding.apply_rules(&mut event);
        assert!(!event.as_mut_log().contains("a.b.c"));
        assert!(!event.as_mut_log().contains("b"));
        assert!(!event.as_mut_log().contains("b[1].x"));
        assert!(!event.as_mut_log().contains("c[0].y"));
        assert!(!event.as_mut_log().contains("d\\.z"));
        assert!(!event.as_mut_log().contains("e.a"));

        assert!(event.as_mut_log().contains("a.b.d"));
        assert!(event.as_mut_log().contains("c[0].x"));
    }

    const YAML_ONLY_FIELD: &str = indoc! {r#"
        encoding:
            codec: "Snoot"
            only_fields: ["a.b.c", "b", "c[0].y", "g\\.z"]
    "#};

    #[test]
    fn test_only() {
        let config: TestConfig = serde_yaml::from_str(YAML_ONLY_FIELD).unwrap();
        config.encoding.validate().unwrap();
        let mut event = Event::new_empty_log();
        {
            let log = event.as_mut_log();
            log.insert_field("a", 1);
            log.insert_field("a.b", 1);
            log.insert_field("a.b.c", 1);
            log.insert_field("a.b.d", 1);
            log.insert_field("b[0]", 1);
            log.insert_field("b[1].x", 1);
            log.insert_field("c[0].x", 1);
            log.insert_field("c[0].y", 1);
            log.insert_field("d.y", 1);
            log.insert_field("d.z", 1);
            log.insert_field("e[0]", 1);
            log.insert_field("e[1]", 1);
            log.insert_field("f\\.z", 1);
            log.insert_field("g\\.z", 1);
            log.insert_field("h", BTreeMap::new());
            log.insert_field("i", Vec::<Value>::new());
        }
        config.encoding.apply_rules(&mut event);
        assert!(event.as_mut_log().contains("a.b.c"));
        assert!(event.as_mut_log().contains("b"));
        assert!(event.as_mut_log().contains("b[1].x"));
        assert!(event.as_mut_log().contains("c[0].y"));
        assert!(event.as_mut_log().contains("g\\.z"));

        assert!(!event.as_mut_log().contains("a.b.d"));
        assert!(!event.as_mut_log().contains("c[0].x"));
        assert!(!event.as_mut_log().contains("d"));
        assert!(!event.as_mut_log().contains("e"));
        assert!(!event.as_mut_log().contains("f"));
        assert!(!event.as_mut_log().contains("h"));
        assert!(!event.as_mut_log().contains("i"));
    }

    const YAML_TIMESTAMP_FORMAT: &str = indoc! {r#"
        encoding:
            codec: "Snoot"
            timestamp_format: "unix"
    "#};

    #[test]
    fn test_timestamp() {
        let config: TestConfig = serde_yaml::from_str(YAML_TIMESTAMP_FORMAT).unwrap();
        config.encoding.validate().unwrap();
        let mut event = Event::from("Demo");
        let timestamp = event
            .as_mut_log()
            .get_field(log_schema().timestamp_key())
            .unwrap()
            .clone();
        let timestamp = timestamp.as_timestamp().unwrap();
        event
            .as_mut_log()
            .insert_field("another", Value::Timestamp(*timestamp));

        config.encoding.apply_rules(&mut event);

        match event
            .as_mut_log()
            .get_field(log_schema().timestamp_key())
            .unwrap()
        {
            Value::Int64(_) => {}
            e => panic!(
                "Timestamp was not transformed into a Unix timestamp. Was {:?}",
                e
            ),
        }
        match event.as_mut_log().get_field("another").unwrap() {
            Value::Int64(_) => {}
            e => panic!(
                "Timestamp was not transformed into a Unix timestamp. Was {:?}",
                e
            ),
        }
    }
}
