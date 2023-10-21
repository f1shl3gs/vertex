use value::{OwnedTargetPath, OwnedValuePath};

use crate::schema::{InstanceType, Metadata, SchemaGenerator, SchemaObject};
use crate::{Configurable, ConfigurableString, GenerateError};

impl Configurable for OwnedValuePath {
    fn generate_schema(_gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        let metadata = Metadata {
            examples: vec![serde_json::Value::String("foo.bar".to_string())],
            ..Default::default()
        };

        Ok(SchemaObject {
            metadata: Some(Box::new(metadata)),
            instance_type: Some(InstanceType::String.into()),
            ..Default::default()
        })
    }
}

impl Configurable for OwnedTargetPath {
    fn generate_schema(_gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        let metadata = Metadata {
            examples: vec![serde_json::Value::String(".foo.bar".to_string())],
            ..Default::default()
        };

        Ok(SchemaObject {
            metadata: Some(Box::new(metadata)),
            instance_type: Some(InstanceType::String.into()),
            ..Default::default()
        })
    }
}

impl ConfigurableString for OwnedTargetPath {}
