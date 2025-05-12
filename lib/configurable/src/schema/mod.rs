mod generator;
mod json_schema;
mod stdlib;

use indexmap::{IndexMap, IndexSet};

pub use generator::SchemaGenerator;
pub use json_schema::{
    ArrayValidation, InstanceType, Metadata, ObjectValidation, RootSchema, SchemaObject,
    SingleOrVec, SubschemaValidation,
};

use crate::configurable::ConfigurableString;
use crate::schema::json_schema::NumberValidation;
use crate::{Configurable, GenerateError};

pub type Map<K, V> = IndexMap<K, V>;
pub type Set<V> = IndexSet<V>;

pub fn generate_root_schema<T>() -> RootSchema
where
    T: Configurable,
{
    let mut generator = SchemaGenerator::default();
    let schema = T::generate_schema(&mut generator);

    RootSchema {
        meta_schema: "https://json-schema.org/draft/2019-09/schema",
        schema,
        definitions: generator.definitions,
    }
}

/// Asserts that the key type `K` generates a string-like schema, suitable for
/// use in maps.
pub fn assert_string_schema_for_map<K, M>(
    generator: &mut SchemaGenerator,
) -> Result<(), GenerateError>
where
    K: ConfigurableString,
{
    let key_schema = generator.subschema_for::<K>();

    // Get a reference to the underlying schema if we're dealing with
    // a reference, or just use what we have if it's the actual definition.
    let underlying_schema = if key_schema.is_ref() {
        generator.dereference(&key_schema)
    } else {
        Some(&key_schema)
    };

    let string_like = match underlying_schema {
        Some(schema_object) => match schema_object.instance_type.as_ref() {
            Some(sov) => match sov {
                // Has to be a string.
                SingleOrVec::Single(it) => **it == InstanceType::String,

                // As long as there's only one instance type, and it's string,
                // we're fine with that, too.
                SingleOrVec::Vec(its) => {
                    its.len() == 1
                        && its
                            .first()
                            .filter(|it| *it == &InstanceType::String)
                            .is_some()
                }
            },
            // We match explicitly, so a lack of declared instance types is not considered
            // valid here.
            None => false,
        },
        // We match explicitly, so boolean schemas aren't considered valid here.
        _ => false,
    };

    if !string_like {
        Err(GenerateError::MapKeyNotStringLike {
            key_type: std::any::type_name::<K>(),
            map_type: std::any::type_name::<M>(),
        })
    } else {
        Ok(())
    }
}
