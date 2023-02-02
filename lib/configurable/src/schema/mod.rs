mod num;

use std::collections::BTreeSet;

use indexmap::IndexMap;
use schemars::gen::{SchemaGenerator, SchemaSettings};
use schemars::schema::{
    InstanceType, NumberValidation, ObjectValidation, RootSchema, Schema, SchemaObject,
    SingleOrVec, SubschemaValidation,
};
use serde::Serialize;
use serde_json::Value;

use crate::configurable::ConfigurableString;
use crate::{Configurable, GenerateError};
use num::ConfigurableNumber;

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
    T: Configurable + Serialize,
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
    T: Configurable + Serialize,
{
    let schema = T::generate_schema(gen)?;
    // TODO: apply metadata?
    Ok(schema)
}

fn get_schema_ref<S: AsRef<str>>(gen: &mut SchemaGenerator, name: S) -> SchemaObject {
    let ref_path = format!("{}{}", gen.settings().definitions_path, name.as_ref());
    SchemaObject::new_ref(ref_path)
}

pub fn generate_root_schema<T>() -> Result<RootSchema, GenerateError>
where
    T: Configurable + Serialize,
{
    let mut schema_gen = SchemaSettings::draft2019_09().into_generator();
    let schema = get_or_generate_schema::<T>(&mut schema_gen)?;

    Ok(RootSchema {
        meta_schema: None,
        schema,
        definitions: schema_gen.take_definitions(),
    })
}

pub fn generate_map_schema<V>(gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError>
where
    V: Configurable + Serialize,
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
    K: ConfigurableString + Serialize,
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
    let mut schema = SchemaObject {
        instance_type: Some(N::class().as_instance_type().into()),
        number: Some(Box::new(NumberValidation {
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
