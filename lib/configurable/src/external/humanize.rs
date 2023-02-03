use humanize::{ByteSize, Duration};
use schemars::gen::SchemaGenerator;
use schemars::schema::SchemaObject;

use crate::schema::generate_string_schema;
use crate::{Configurable, GenerateError};

impl Configurable for ByteSize {
    fn generate_schema(_gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        Ok(generate_string_schema())
    }
}

impl Configurable for Duration {
    fn generate_schema(_gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        Ok(generate_string_schema())
    }
}
