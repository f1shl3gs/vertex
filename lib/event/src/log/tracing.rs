use std::fmt::Debug;
use std::sync::OnceLock;

use chrono::Utc;
use tracing::field::Field;
use value::{event_path, owned_value_path, OwnedTargetPath, Value};

use super::LogRecord;

impl tracing::field::Visit for LogRecord {
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.insert(event_path!(field.name()), value);
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        let field_path = event_path!(field.name());

        match TryInto::<i64>::try_into(value) {
            Ok(value) => self.insert(field_path, value),
            Err(_) => self.insert(field_path, value.to_string()),
        };
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.insert(event_path!(field.name()), value);
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.insert(event_path!(field.name()), value.to_string());
    }

    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        self.insert(event_path!(field.name()), format!("{value:?}"));
    }
}
// Tracing owned target paths used for tracing to log event conversions.
struct TracingTargetPaths {
    pub(crate) timestamp: OwnedTargetPath,
    pub(crate) kind: OwnedTargetPath,
    pub(crate) module_path: OwnedTargetPath,
    pub(crate) level: OwnedTargetPath,
    pub(crate) target: OwnedTargetPath,
}

impl TracingTargetPaths {
    fn new() -> Self {
        Self {
            timestamp: OwnedTargetPath::event(owned_value_path!("timestamp")),
            kind: OwnedTargetPath::event(owned_value_path!("metadata", "kind")),
            level: OwnedTargetPath::event(owned_value_path!("metadata", "level")),
            module_path: OwnedTargetPath::event(owned_value_path!("metadata", "module_path")),
            target: OwnedTargetPath::event(owned_value_path!("metadata", "target")),
        }
    }
}

static TRACING_TARGET_PATHS: OnceLock<TracingTargetPaths> = OnceLock::new();

impl From<&tracing::Event<'_>> for LogRecord {
    fn from(event: &tracing::Event<'_>) -> Self {
        let mut log = LogRecord::default();
        event.record(&mut log);

        let target_paths = TRACING_TARGET_PATHS.get_or_init(TracingTargetPaths::new);

        log.insert(&target_paths.timestamp, Utc::now());

        let meta = event.metadata();
        log.insert(&target_paths.level, meta.level().to_string());
        log.insert(&target_paths.target, meta.target().to_string());
        log.insert(
            &target_paths.module_path,
            meta.module_path()
                .map_or(Value::Null, |mp| Value::Bytes(mp.to_string().into())),
        );
        log.insert(
            &target_paths.kind,
            if meta.is_event() {
                Value::Bytes("event".to_string().into())
            } else if meta.is_span() {
                Value::Bytes("span".to_string().into())
            } else {
                Value::Null
            },
        );

        log
    }
}
