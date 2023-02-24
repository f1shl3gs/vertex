use crate::{
    schema::{generate_string_schema, SchemaGenerator, SchemaObject},
    Configurable, GenerateError,
};

impl Configurable for &'static encoding_rs::Encoding {
    fn reference() -> Option<&'static str> {
        Some("encoding_rs::Encoding")
    }

    fn generate_schema(_: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        let mut schema = generate_string_schema();
        let metadata = schema.metadata();
        metadata.description = Some(
            "An encoding as defined in the [Encoding Standard](https://encoding.spec.whatwg.org/)."
                .to_string(),
        );

        Ok(schema)
    }
}
