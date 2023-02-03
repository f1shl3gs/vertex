use schemars::gen::SchemaGenerator;
use schemars::schema::SchemaObject;

use crate::schema::generate_string_schema;
use crate::{Configurable, GenerateError};

impl Configurable for url::Url {
    fn generate_schema(_gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        let mut schema = generate_string_schema();

        schema.format = Some("uri".to_string());

        Ok(schema)
    }
}
