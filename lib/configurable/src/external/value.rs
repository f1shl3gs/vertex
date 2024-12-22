use value::{OwnedTargetPath, OwnedValuePath};

use crate::schema::{InstanceType, Metadata, SchemaGenerator, SchemaObject};
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
