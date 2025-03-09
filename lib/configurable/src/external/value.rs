use value::{OwnedTargetPath, OwnedValuePath, Value};

use crate::schema::{InstanceType, Metadata, SchemaGenerator, SchemaObject, SingleOrVec};
use crate::{Configurable, ConfigurableString};

impl Configurable for OwnedValuePath {
    fn generate_schema(_gen: &mut SchemaGenerator) -> SchemaObject {
        let metadata = Metadata {
            examples: vec![serde_json::Value::String("foo.bar".to_string())],
            ..Default::default()
        };

        SchemaObject {
            metadata: Some(Box::new(metadata)),
            instance_type: Some(InstanceType::String.into()),
            ..Default::default()
        }
    }
}

impl Configurable for OwnedTargetPath {
    fn generate_schema(_gen: &mut SchemaGenerator) -> SchemaObject {
        let metadata = Metadata {
            examples: vec![serde_json::Value::String(".foo.bar".to_string())],
            ..Default::default()
        };

        SchemaObject {
            metadata: Some(Box::new(metadata)),
            instance_type: Some(InstanceType::String.into()),
            ..Default::default()
        }
    }
}

impl ConfigurableString for OwnedTargetPath {}

impl Configurable for Value {
    fn generate_schema(_: &mut SchemaGenerator) -> SchemaObject {
        SchemaObject {
            instance_type: Some(SingleOrVec::Vec(vec![
                InstanceType::Array,
                InstanceType::Boolean,
                InstanceType::Integer,
                InstanceType::Null,
                InstanceType::String,
                InstanceType::Object,
            ])),
            ..Default::default()
        }
    }
}
