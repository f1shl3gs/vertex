use lookup::OwnedPath;

use crate::schema::{generate_string_schema, SchemaGenerator, SchemaObject};
use crate::{Configurable, GenerateError};

impl Configurable for OwnedPath {
    fn generate_schema(_gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        Ok(generate_string_schema())
    }
}
