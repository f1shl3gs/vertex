use indexmap::IndexMap;
use serde::Serialize;

use crate::Configurable;
use crate::configurable::ConfigurableString;
use crate::schema::{
    InstanceType, ObjectValidation, SchemaGenerator, SchemaObject, assert_string_schema_for_map,
};

impl<K, V> Configurable for IndexMap<K, V>
where
    K: ConfigurableString + Serialize + Ord,
    V: Configurable + Serialize,
{
    fn required() -> bool {
        false
    }

    fn generate_schema(generator: &mut SchemaGenerator) -> SchemaObject {
        assert_string_schema_for_map::<K, Self>(generator).expect("key must be string like");

        SchemaObject {
            instance_type: Some(InstanceType::Object.into()),
            object: Some(Box::new(ObjectValidation {
                additional_properties: Some(Box::new(generator.subschema_for::<V>().into())),
                ..Default::default()
            })),
            ..Default::default()
        }
    }
}
