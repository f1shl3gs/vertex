use event::tags::{Key, Value};

use crate::schema::{generate_string_schema, InstanceType, SchemaGenerator, SchemaObject};
use crate::{Configurable, GenerateError};

impl Configurable for Key {
    fn generate_schema(_gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        Ok(generate_string_schema())
    }
}

impl Configurable for Value {
    fn generate_schema(_gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        Ok(SchemaObject {
            instance_type: Some(
                vec![
                    InstanceType::Boolean,
                    InstanceType::Integer,
                    InstanceType::Number,
                    InstanceType::String,
                    InstanceType::Array,
                ]
                .into(),
            ),
            ..Default::default()
        })
    }
}
