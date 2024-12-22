use crate::schema::{SchemaGenerator, SchemaObject};
use crate::Configurable;

impl Configurable for chrono_tz::Tz {
    fn generate_schema(gen: &mut SchemaGenerator) -> SchemaObject {
        let mut schema = String::generate_schema(gen);
        let metadta = schema.metadata();
        metadta.description = Some("An IANA timezone.");

        schema
    }
}
