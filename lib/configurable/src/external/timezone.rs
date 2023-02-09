use crate::{
    schema::generate_string_schema,
    schemars::{gen::SchemaGenerator, schema::SchemaObject},
    Configurable, GenerateError,
};

impl Configurable for chrono_tz::Tz {
    fn generate_schema(_: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        let mut schema = generate_string_schema();
        let metadta = schema.metadata();
        metadta.description = Some("An IANA timezone.".to_string());

        Ok(schema)
    }
}
