use crate::schema::{generate_string_schema, SchemaGenerator, SchemaObject};
use crate::{Configurable, GenerateError};

impl Configurable for regex::bytes::Regex {
    fn generate_schema(_gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        Ok(generate_string_schema())
    }
}
