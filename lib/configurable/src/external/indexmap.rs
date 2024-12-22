use indexmap::IndexMap;
use serde::Serialize;

use crate::configurable::ConfigurableString;
use crate::schema::{
    assert_string_schema_for_map, InstanceType, ObjectValidation, SchemaGenerator, SchemaObject,
};
use crate::Configurable;

impl<K, V> Configurable for IndexMap<K, V>
where
    K: ConfigurableString + Serialize + Ord,
    V: Configurable + Serialize,
{
    fn required() -> bool {
        false
    }

    fn generate_schema(gen: &mut SchemaGenerator) -> SchemaObject {
        assert_string_schema_for_map::<K, Self>(gen).expect("key must be string like");

        SchemaObject {
            instance_type: Some(InstanceType::Object.into()),
            object: Some(Box::new(ObjectValidation {
                additional_properties: Some(Box::new(gen.subschema_for::<V>().into())),
                ..Default::default()
            })),
            ..Default::default()
        }
    }
}
