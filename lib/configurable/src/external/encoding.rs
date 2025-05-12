use crate::Configurable;
use crate::schema::{SchemaGenerator, SchemaObject};

impl Configurable for &'static encoding_rs::Encoding {
    fn reference() -> Option<&'static str> {
        Some("encoding_rs::Encoding")
    }

    fn generate_schema(generator: &mut SchemaGenerator) -> SchemaObject {
        let mut schema = String::generate_schema(generator);
        schema.metadata.description = Some(
            "An encoding as defined in the [Encoding Standard](https://encoding.spec.whatwg.org/).",
        );

        schema
    }
}
