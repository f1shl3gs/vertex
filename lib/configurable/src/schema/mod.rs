mod gen;
mod json_schema;
mod num;
mod stdlib;
mod visit;

use std::collections::BTreeSet;

use crate::configurable::ConfigurableString;
use crate::schema::json_schema::NumberValidation;
use crate::{Configurable, GenerateError};
pub use gen::{SchemaGenerator, SchemaSettings};
use indexmap::IndexMap;
pub use json_schema::{
    ArrayValidation, InstanceType, Metadata, ObjectValidation, RootSchema, Schema, SchemaObject,
    SingleOrVec, SubschemaValidation,
};
use num::ConfigurableNumber;
use serde_json::Value;

pub type Map<K, V> = IndexMap<K, V>;
pub type Set<V> = BTreeSet<V>;

pub fn generate_struct_schema(
    properties: IndexMap<String, SchemaObject>,
    required: BTreeSet<String>,
    additional_properties: Option<Box<Schema>>,
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
            additional_properties,
            ..Default::default()
        })),
        ..Default::default()
    }
}

pub fn generate_empty_struct_schema() -> SchemaObject {
    SchemaObject {
        instance_type: Some(InstanceType::Object.into()),
        object: Some(Box::new(ObjectValidation {
            properties: Default::default(),
            ..Default::default()
        })),
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

pub fn get_or_generate_schema<T>(gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError>
where
    T: Configurable,
{
    let schema = match T::reference() {
        Some(name) => {
            if !gen.definitions().contains_key(name) {
                gen.definitions_mut()
                    .insert(name.to_string(), Schema::Bool(false));

                let schema = generate_baseline_schema::<T>(gen)?;

                gen.definitions_mut()
                    .insert(name.to_string(), Schema::Object(schema));
            }

            get_schema_ref(gen, name)
        }
        None => T::generate_schema(gen)?,
    };

    Ok(schema)
}

pub fn generate_baseline_schema<T>(gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError>
where
    T: Configurable,
{
    let schema = T::generate_schema(gen)?;
    // TODO: apply metadata?
    Ok(schema)
}

fn get_schema_ref<S: AsRef<str>>(gen: &mut SchemaGenerator, name: S) -> SchemaObject {
    let ref_path = format!("{}{}", gen.settings().definitions_path(), name.as_ref());
    SchemaObject::new_ref(ref_path)
}

pub fn generate_root_schema<T>() -> Result<RootSchema, GenerateError>
where
    T: Configurable,
{
    let mut schema_gen = SchemaSettings::new().into_generator();
    let schema = get_or_generate_schema::<T>(&mut schema_gen)?;

    Ok(schema_gen.into_root_schema(schema))
}

pub fn generate_map_schema<V>(gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError>
where
    V: Configurable,
{
    let element_schema = get_or_generate_schema::<V>(gen)?;

    Ok(SchemaObject {
        instance_type: Some(InstanceType::Object.into()),
        object: Some(Box::new(ObjectValidation {
            additional_properties: Some(Box::new(element_schema.into())),
            ..Default::default()
        })),
        ..Default::default()
    })
}

/// Asserts that the key type `K` generates a string-like schema, suitable for
/// use in maps.
pub fn assert_string_schema_for_map<K, M>(gen: &mut SchemaGenerator) -> Result<(), GenerateError>
where
    K: ConfigurableString,
{
    let key_schema = get_or_generate_schema::<K>(gen)?;
    let wrapped_schema = Schema::Object(key_schema);

    // Get a reference to the underlying schema if we're dealing with
    // a reference, or just use what we have if it's the actual definition.
    let underlying_schema = if wrapped_schema.is_ref() {
        gen.dereference(&wrapped_schema)
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
                            .get(0)
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

pub fn generate_one_of_schema(subschemas: &[SchemaObject]) -> SchemaObject {
    let subschemas = subschemas
        .iter()
        .map(|s| Schema::Object(s.clone()))
        .collect::<Vec<_>>();

    SchemaObject {
        subschemas: Some(Box::new(SubschemaValidation {
            one_of: Some(subschemas),
            ..Default::default()
        })),
        ..Default::default()
    }
}

pub fn make_schema_optional(schema: &mut SchemaObject) -> Result<(), GenerateError> {
    // We do a little dance here to add an ad
    match schema.instance_type.as_mut() {
        None => match schema.subschemas.as_mut() {
            None => return Err(GenerateError::InvalidOptionalSchema),
            Some(subschemas) => {
                if let Some(any_of) = subschemas.any_of.as_mut() {
                    any_of.push(Schema::Object(generate_null_schema()));
                } else if let Some(one_of) = subschemas.one_of.as_mut() {
                    one_of.push(Schema::Object(generate_null_schema()));
                } else if subschemas.all_of.is_some() {
                    // If we're dealing with an all-of schema, we have to build a new
                    // one-of schema where the two choices are either the `null` schema,
                    // or a subschema comprised of the all-of subschemas.
                    let all_of = subschemas
                        .all_of
                        .take()
                        .expect("all-of subschemas must be present here");
                    let new_all_of_schema = SchemaObject {
                        subschemas: Some(Box::new(SubschemaValidation {
                            all_of: Some(all_of),
                            ..Default::default()
                        })),
                        ..Default::default()
                    };

                    subschemas.one_of = Some(vec![
                        Schema::Object(generate_null_schema()),
                        Schema::Object(new_all_of_schema),
                    ]);
                } else {
                    return Err(GenerateError::InvalidOptionalSchema);
                }
            }
        },

        Some(sov) => match sov {
            SingleOrVec::Single(ty) if **ty != InstanceType::Null => {
                *sov = vec![**ty, InstanceType::Null].into()
            }
            SingleOrVec::Vec(ty) if !ty.contains(&InstanceType::Null) => {
                ty.push(InstanceType::Null)
            }
            _ => {}
        },
    }

    Ok(())
}

pub fn generate_null_schema() -> SchemaObject {
    SchemaObject {
        instance_type: Some(InstanceType::Null.into()),
        ..Default::default()
    }
}

pub fn generate_const_string_schema(value: String) -> SchemaObject {
    SchemaObject {
        const_value: Some(Value::String(value)),
        ..Default::default()
    }
}

pub fn generate_bool_schema() -> SchemaObject {
    SchemaObject {
        instance_type: Some(InstanceType::Boolean.into()),
        ..Default::default()
    }
}

pub fn generate_string_schema() -> SchemaObject {
    SchemaObject {
        instance_type: Some(InstanceType::String.into()),
        ..Default::default()
    }
}

pub fn generate_array_schema<T>(gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError>
where
    T: Configurable,
{
    // Generate the actual schema for the element type `T`.
    let element_schema = get_or_generate_schema::<T>(gen)?;

    Ok(SchemaObject {
        instance_type: Some(InstanceType::Array.into()),
        array: Some(Box::new(ArrayValidation {
            items: Some(SingleOrVec::Single(Box::new(element_schema.into()))),
            ..Default::default()
        })),
        ..Default::default()
    })
}

pub fn generate_internal_tagged_variant_schema(
    tag: String,
    value_schema: SchemaObject,
) -> SchemaObject {
    let mut properties = IndexMap::new();
    properties.insert(tag.clone(), value_schema);

    let mut required = BTreeSet::new();
    required.insert(tag);

    generate_struct_schema(properties, required, None)
}
