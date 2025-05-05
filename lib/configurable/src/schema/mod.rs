mod generator;
mod json_schema;
mod num;
mod stdlib;
mod visit;

use std::collections::BTreeSet;

use indexmap::IndexMap;
use num::ConfigurableNumber;
use serde_json::Value;

pub use generator::{SchemaGenerator, SchemaSettings};
pub use json_schema::{
    ArrayValidation, InstanceType, Metadata, ObjectValidation, RootSchema, Schema, SchemaObject,
    SingleOrVec, SubschemaValidation,
};

use crate::configurable::ConfigurableString;
use crate::schema::json_schema::NumberValidation;
use crate::{Configurable, GenerateError};

pub type Map<K, V> = IndexMap<K, V>;
pub type Set<V> = BTreeSet<V>;

pub fn generate_struct_schema(
    properties: IndexMap<&'static str, SchemaObject>,
    required: BTreeSet<&'static str>,
    description: Option<&'static str>,
) -> SchemaObject {
    let properties = properties
        .into_iter()
        .map(|(k, v)| (k, Schema::Object(v)))
        .collect();

    SchemaObject {
        instance_type: Some(InstanceType::Object.into()),
        object: Some(Box::new(ObjectValidation {
            properties,
            required,
            ..Default::default()
        })),
        metadata: Metadata {
            description,
            ..Default::default()
        },
        ..Default::default()
    }
}

#[inline]
pub fn generate_empty_struct_schema() -> SchemaObject {
    SchemaObject {
        instance_type: Some(InstanceType::Object.into()),
        object: Some(
            ObjectValidation {
                properties: Default::default(),
                ..Default::default()
            }
            .into(),
        ),
        ..Default::default()
    }
}

pub fn convert_to_flattened_schema(primary: &mut SchemaObject, mut subschema: Vec<SchemaObject>) {
    let primary_subschema = std::mem::take(primary);
    subschema.insert(0, primary_subschema);

    let all_of_schemas = subschema.into_iter().map(Schema::Object).collect();

    primary.subschemas = Some(Box::new(SubschemaValidation {
        all_of: Some(all_of_schemas),
        ..Default::default()
    }));
}

pub fn generate_root_schema<T>() -> RootSchema
where
    T: Configurable,
{
    let mut generator = SchemaSettings::new().into_generator();
    let schema = generator.subschema_for::<T>();

    generator.into_root_schema(schema)
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
    let wrapped_schema = Schema::Object(key_schema);

    // Get a reference to the underlying schema if we're dealing with
    // a reference, or just use what we have if it's the actual definition.
    let underlying_schema = if wrapped_schema.is_ref() {
        generator.dereference(&wrapped_schema)
    } else {
        Some(&wrapped_schema)
    };

    let string_like = match underlying_schema {
        Some(Schema::Object(schema_object)) => match schema_object.instance_type.as_ref() {
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

pub fn generate_number_schema<N>() -> SchemaObject
where
    N: Configurable + ConfigurableNumber,
{
    let minimum = N::get_enforced_min_bound();
    let maximum = N::get_enforced_max_bound();

    let mut schema = SchemaObject {
        instance_type: Some(N::class().as_instance_type().into()),
        number: Some(Box::new(NumberValidation {
            minimum: Some(minimum),
            maximum: Some(maximum),
            ..Default::default()
        })),
        ..Default::default()
    };

    if N::requires_nonzero_exclusion() {
        schema.subschemas = Some(Box::new(SubschemaValidation {
            not: Some(Box::new(Schema::Object(SchemaObject {
                const_value: Some(Value::Number(0.into())),
                ..Default::default()
            }))),
            ..Default::default()
        }));
    }

    schema
}

#[inline]
pub fn generate_one_of_schema(subschemas: Vec<SchemaObject>) -> SchemaObject {
    let subschemas = subschemas
        .into_iter()
        .map(Schema::Object)
        .collect::<Vec<_>>();

    SchemaObject {
        subschemas: Some(Box::new(SubschemaValidation {
            one_of: Some(subschemas),
            ..Default::default()
        })),
        ..Default::default()
    }
}

#[inline]
pub fn generate_null_schema() -> SchemaObject {
    SchemaObject {
        instance_type: Some(InstanceType::Null.into()),
        ..Default::default()
    }
}

#[inline]
pub fn generate_const_string_schema(value: String) -> SchemaObject {
    SchemaObject {
        const_value: Some(Value::String(value)),
        ..Default::default()
    }
}

pub fn generate_internal_tagged_variant_schema(
    tag: &'static str,
    value_schema: SchemaObject,
) -> SchemaObject {
    let mut properties = IndexMap::new();
    properties.insert(tag, value_schema);

    let mut required = BTreeSet::new();
    required.insert(tag);

    generate_struct_schema(properties, required, None)
}
