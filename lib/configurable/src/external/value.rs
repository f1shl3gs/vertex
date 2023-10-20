use value::{OwnedTargetPath, OwnedValuePath};

use crate::schema::{generate_string_schema, SchemaGenerator, SchemaObject};
use crate::{Configurable, ConfigurableString, GenerateError};

impl Configurable for OwnedValuePath {
    fn generate_schema(_gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        Ok(generate_string_schema())
    }
}

impl Configurable for OwnedTargetPath {
    fn generate_schema(_gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        Ok(generate_string_schema())
    }
}

impl ConfigurableString for OwnedTargetPath {}
