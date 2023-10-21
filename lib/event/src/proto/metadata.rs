use std::collections::BTreeMap;

use value::Value;

use super::log::{decode_value, encode_value};
use crate::proto::Metadata;
use crate::EventMetadata;

impl From<EventMetadata> for Metadata {
    fn from(value: EventMetadata) -> Self {
        Self {
            value: Some(encode_value(value.value)),
            source_id: value.source_id.map(|v| v.to_string()),
            source_type: value.source_type.map(|v| v.to_string()),
        }
    }
}

impl From<Metadata> for EventMetadata {
    fn from(value: Metadata) -> Self {
        let mut metadata = EventMetadata::default();

        if let Some(pv) = value.value {
            metadata.value = decode_value(pv).unwrap_or_else(|| Value::Object(BTreeMap::new()));
        }

        metadata.source_id = value.source_id.map(|v| v.into());
        metadata.source_type = value.source_type.map(|v| v.into());

        metadata
    }
}
