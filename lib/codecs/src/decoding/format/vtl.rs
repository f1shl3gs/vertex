use std::fmt::Debug;

use bytes::Bytes;
use configurable::Configurable;
use event::log::path::PathPrefix;
use event::log::{OwnedTargetPath, Value};
use event::{EventMetadata, Events, LogRecord};
use serde::{Deserialize, Serialize};
use vtl::{ContextError, Program, Target};

use super::{DeserializeError, Deserializer};
use crate::error::BuildError;

/// Config used to build a `VtlDeserializer`
#[derive(Clone, Configurable, Debug, Deserialize, Serialize)]
pub struct VtlDeserializerConfig {
    /// The `VTL` program to execute for each event.
    /// Note that the final contents of the `.` target will be used as
    /// the decoding result.
    source: String,
}

impl VtlDeserializerConfig {
    /// Build the `VtlDeserializer` from this configuration
    pub fn build(&self) -> Result<VtlDeserializer, BuildError> {
        let program = vtl::compile(&self.source).map_err(BuildError::Vtl)?;

        Ok(VtlDeserializer { program })
    }
}

/// Deserializer that builds `Events` from a bytes frame containing logs compatible with VRL
#[derive(Clone)]
pub struct VtlDeserializer {
    program: Program,
}

impl Debug for VtlDeserializer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VtlDeserializer").finish()
    }
}

impl Deserializer for VtlDeserializer {
    fn parse(&self, buf: Bytes) -> Result<Events, DeserializeError> {
        let mut target = BytesTarget::new(buf);

        let _ = self.program.run(&mut target)?;

        match target.data {
            Value::Array(values) => {
                let metadata = target.metadata;
                let mut logs = Vec::with_capacity(values.len());

                let last = values.len() - 1;
                for (index, value) in values.into_iter().enumerate() {
                    if index == last {
                        logs.push(LogRecord::from_parts(
                            EventMetadata::default_with_value(metadata),
                            value,
                        ));

                        // `break` is not necessary for human, cause this is the last element,
                        // and it's safe to put it to EventMetadata
                        //
                        // But the compiler can't understand
                        break;
                    } else {
                        logs.push(LogRecord::from_parts(
                            EventMetadata::default_with_value(metadata.clone()),
                            value,
                        ))
                    };
                }

                Ok(Events::Logs(logs))
            }
            value => {
                let metadata = target.metadata;
                Ok(Events::Logs(vec![LogRecord::from_parts(
                    EventMetadata::default_with_value(metadata),
                    value,
                )]))
            }
        }
    }
}

#[derive(Debug)]
struct BytesTarget {
    data: Value,
    metadata: Value,
}

impl BytesTarget {
    #[inline]
    fn new(data: Bytes) -> Self {
        Self {
            data: Value::Bytes(data),
            metadata: Value::Object(Default::default()),
        }
    }
}

impl Target for BytesTarget {
    fn insert(&mut self, path: &OwnedTargetPath, value: Value) -> Result<(), ContextError> {
        match path.prefix {
            PathPrefix::Event => self.data.insert(&path.path, value),
            PathPrefix::Metadata => self.metadata.insert(&path.path, value),
        };

        Ok(())
    }

    fn get(&mut self, path: &OwnedTargetPath) -> Result<Option<&Value>, ContextError> {
        let value = match path.prefix {
            PathPrefix::Event => self.data.get(&path.path),
            PathPrefix::Metadata => self.metadata.get(&path.path),
        };

        Ok(value)
    }

    fn get_mut(&mut self, path: &OwnedTargetPath) -> Result<Option<&mut Value>, ContextError> {
        let value = match path.prefix {
            PathPrefix::Event => self.data.get_mut(&path.path),
            PathPrefix::Metadata => self.metadata.get_mut(&path.path),
        };

        Ok(value)
    }

    fn remove(
        &mut self,
        path: &OwnedTargetPath,
        compact: bool,
    ) -> Result<Option<Value>, ContextError> {
        let value = match path.prefix {
            PathPrefix::Event => self.data.remove(&path.path, compact),
            PathPrefix::Metadata => self.metadata.remove(&path.path, compact),
        };

        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use value::value;

    #[test]
    fn single() {
        let input = Bytes::from(r#"{"foo":123}"#);
        let config = VtlDeserializerConfig {
            source: r#"
., err = parse_json(.)
"#
            .into(),
        };
        let deserializer = config.build().unwrap();

        let output = deserializer.parse(input).unwrap();

        assert_eq!(output.len(), 1);
        let log = output.into_logs().unwrap().pop().unwrap();
        assert_eq!(log.value(), &value!({ "foo": 123 }));
    }

    #[test]
    fn multiple() {
        let input = Bytes::from(
            r#"
[
{"foo":123},
{"foo":456}
]
"#,
        );
        let config = VtlDeserializerConfig {
            source: r#"
., err = parse_json(.)
"#
            .into(),
        };
        let deserializer = config.build().unwrap();

        let output = deserializer.parse(input).unwrap();

        assert_eq!(output.len(), 2);
        let mut logs = output.into_logs().unwrap().into_iter();

        assert_eq!(logs.next().unwrap().value(), &value!({ "foo": 123 }));
        assert_eq!(logs.next().unwrap().value(), &value!({ "foo": 456 }));
    }

    #[test]
    fn metadata() {
        let input = Bytes::from(r#"{"foo":123}"#);
        let config = VtlDeserializerConfig {
            source: r#"
., err = parse_json(.)
%bar = 456
"#
            .into(),
        };
        let deserializer = config.build().unwrap();

        let output = deserializer.parse(input).unwrap();

        assert_eq!(output.len(), 1);
        let log = output.into_logs().unwrap().pop().unwrap();
        assert_eq!(log.value(), &value!({ "foo": 123 }));
        assert_eq!(log.metadata().value(), &value!({ "bar": 456 }));
    }

    #[test]
    fn multiple_event_with_metadata() {
        let input = Bytes::from(
            r#"
[
{"foo":123},
{"foo":456}
]
"#,
        );
        let config = VtlDeserializerConfig {
            source: r#"
., err = parse_json(.)
%bar = 456
"#
            .into(),
        };
        let deserializer = config.build().unwrap();

        let output = deserializer.parse(input).unwrap();

        assert_eq!(output.len(), 2);
        let mut logs = output.into_logs().unwrap().into_iter();

        let log1 = logs.next().unwrap();
        assert_eq!(log1.value(), &value!({ "foo": 123 }));
        assert_eq!(log1.metadata().value(), &value!({ "bar": 456 }));

        let log2 = logs.next().unwrap();
        assert_eq!(log2.value(), &value!({ "foo": 456 }));
        assert_eq!(log2.metadata().value(), &value!({ "bar": 456 }));
    }
}
