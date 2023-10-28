use crate::schema::{InstanceType, Metadata, SchemaGenerator, SchemaObject};
use crate::{Configurable, GenerateError};

impl Configurable for condition::Expression {
    fn generate_schema(_gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        let metadata = Metadata {
            examples: vec![serde_json::Value::String(".foo contains bar".to_string())],
            ..Default::default()
        };

        Ok(SchemaObject {
            metadata: Some(Box::new(metadata)),
            instance_type: Some(InstanceType::String.into()),
            ..Default::default()
        })
    }
}
