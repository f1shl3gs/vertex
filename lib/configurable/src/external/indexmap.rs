use crate::schema::{assert_string_schema_for_map, generate_map_schema};
use crate::stdlib::ConfigurableString;
use crate::{Configurable, GenerateError};
use indexmap::IndexMap;
use schemars::gen::SchemaGenerator;
use schemars::schema::SchemaObject;
use serde::Serialize;

impl<K, V> Configurable for IndexMap<K, V>
where
    K: ConfigurableString + Serialize + Ord,
    V: Configurable + Serialize,
{
    fn required() -> bool {
        false
    }

    fn generate_schema(gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        assert_string_schema_for_map::<K, Self>(gen)?;

        generate_map_schema::<V>(gen)
    }
}
