use indexmap::IndexMap;
use serde::Serialize;

use crate::configurable::ConfigurableString;
use crate::schema::{
    assert_string_schema_for_map, generate_map_schema, SchemaGenerator, SchemaObject,
};
use crate::{Configurable, GenerateError};

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
