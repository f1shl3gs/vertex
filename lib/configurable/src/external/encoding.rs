use crate::schema::{SchemaGenerator, SchemaObject};
use crate::Configurable;

impl Configurable for &'static encoding_rs::Encoding {
    fn reference() -> Option<&'static str> {
        Some("encoding_rs::Encoding")
    }

    fn generate_schema(gen: &mut SchemaGenerator) -> SchemaObject {
        let mut schema = String::generate_schema(gen);
        let metadata = schema.metadata();
        metadata.description = Some(
            "An encoding as defined in the [Encoding Standard](https://encoding.spec.whatwg.org/).",
        );

        schema
    }
}
