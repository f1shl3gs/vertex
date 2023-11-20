use std::collections::BTreeMap;

use event::tags::{Key, Tags, Value};

use crate::schema::{
    generate_map_schema, generate_string_schema, InstanceType, SchemaGenerator, SchemaObject,
};
use crate::{Configurable, ConfigurableString, GenerateError};

impl Configurable for Key {
    fn generate_schema(_gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        Ok(generate_string_schema())
    }
}

impl ConfigurableString for Key {}

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

impl Configurable for Tags {
    fn generate_schema(gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        generate_map_schema::<BTreeMap<Key, Value>>(gen)
    }
}
