use crate::Configurable;
use crate::schema::{SchemaGenerator, SchemaObject};

impl Configurable for chrono_tz::Tz {
    fn generate_schema(generator: &mut SchemaGenerator) -> SchemaObject {
        let mut schema = String::generate_schema(generator);
        let metadta = schema.metadata();
        metadta.description = Some("An IANA timezone.");

        schema
    }
}
