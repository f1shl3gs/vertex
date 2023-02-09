use no_proxy::NoProxy;
use schemars::gen::SchemaGenerator;
use schemars::schema::SchemaObject;

use crate::schema::generate_array_schema;
use crate::{Configurable, GenerateError};

impl Configurable for NoProxy {
    fn generate_schema(gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        generate_array_schema::<String>(gen)
    }
}
