use std::collections::BTreeMap;
use std::time::Duration;

use schemars::gen::SchemaGenerator;
use schemars::schema::{ArrayValidation, InstanceType, SchemaObject, SingleOrVec};
use serde::Serialize;

use crate::configurable::ConfigurableString;
use crate::schema::generate_number_schema;
use crate::schema::{
    assert_string_schema_for_map, generate_baseline_schema, generate_bool_schema,
    generate_map_schema, get_or_generate_schema, make_schema_optional,
};
use crate::{Configurable, GenerateError};

impl<K, V> Configurable for BTreeMap<K, V>
where
    K: ConfigurableString + Serialize + Ord,
    V: Configurable + Serialize,
{
    fn required() -> bool {
        // A map with required fields would be... an object. So if you want that,
        // make a struct instead, not a map.
        false
    }

    fn generate_schema(gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        // Make sure our key type is _truly_ a string schema.
        assert_string_schema_for_map::<K, Self>(gen)?;

        generate_map_schema::<V>(gen)
    }
}

impl Configurable for String {
    fn generate_schema(_gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        Ok(SchemaObject {
            instance_type: Some(InstanceType::String.into()),
            ..Default::default()
        })
    }
}

impl Configurable for bool {
    fn generate_schema(_gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        Ok(generate_bool_schema())
    }
}

impl<T> Configurable for Option<T>
where
    T: Configurable + Serialize,
{
    fn reference() -> Option<&'static str> {
        match T::reference() {
            Some(_) => Some(std::any::type_name::<Self>()),
            None => None,
        }
    }

    fn required() -> bool {
        false
    }

    fn generate_schema(gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        let mut inner_schema = generate_baseline_schema::<T>(gen)?;
        make_schema_optional(&mut inner_schema)?;

        Ok(inner_schema)
    }
}

// Array
impl<T> Configurable for Vec<T>
where
    T: Configurable + Serialize,
{
    fn generate_schema(gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        generate_array_schema::<T>(gen)
    }
}

impl Configurable for Duration {
    fn generate_schema(_gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        Ok(SchemaObject {
            instance_type: Some(InstanceType::String.into()),
            ..Default::default()
        })
    }
}

fn generate_array_schema<T>(gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError>
where
    T: Configurable + Serialize,
{
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

// Numbers.
macro_rules! impl_configurable_numeric {
	($($ty:ty),+) => {
		$(
			impl Configurable for $ty {
                // fn metadata() -> Metadata<Self> {
                //     let mut metadata = Metadata::default();
                //     let numeric_type = <Self as ConfigurableNumber>::class();
                //     metadata.add_custom_attribute(CustomAttribute::kv("docs::numeric_type", numeric_type));
                //
                //     metadata
                // }

                // fn validate_metadata(metadata: &Metadata<Self>) -> Result<(), GenerateError> {
                //     $crate::__ensure_numeric_validation_bounds::<Self>(metadata)
                // }

				fn generate_schema(_: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
					Ok(generate_number_schema::<Self>())
				}
			}
		)+
	};
}

impl_configurable_numeric!(
    u8,
    u16,
    u32,
    u64,
    usize,
    i8,
    i16,
    i32,
    i64,
    isize,
    f32,
    f64,
    std::num::NonZeroU8,
    std::num::NonZeroU16,
    std::num::NonZeroU32,
    std::num::NonZeroU64,
    std::num::NonZeroI8,
    std::num::NonZeroI16,
    std::num::NonZeroI32,
    std::num::NonZeroI64,
    std::num::NonZeroUsize
);
