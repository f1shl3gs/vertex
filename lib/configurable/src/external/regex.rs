use crate::Configurable;
use crate::schema::{InstanceType, SchemaGenerator, SchemaObject};

impl Configurable for regex::Regex {
    fn generate_schema(_gen: &mut SchemaGenerator) -> SchemaObject {
        SchemaObject {
            instance_type: Some(InstanceType::String.into()),
            format: Some("regex"),
            ..Default::default()
        }
    }
}
