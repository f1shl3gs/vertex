use no_proxy::NoProxy;

use crate::schema::{generate_array_schema, SchemaGenerator, SchemaObject};
use crate::{Configurable, GenerateError};

impl Configurable for NoProxy {
    fn generate_schema(gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        generate_array_schema::<String>(gen)
    }
}
