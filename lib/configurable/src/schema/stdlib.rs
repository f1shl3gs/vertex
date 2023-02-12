use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use schemars::gen::SchemaGenerator;
use schemars::schema::{InstanceType, Metadata, SchemaObject};
use serde_json::Value;

use crate::configurable::ConfigurableString;
use crate::schema::{
    assert_string_schema_for_map, generate_array_schema, generate_baseline_schema,
    generate_bool_schema, generate_map_schema, generate_number_schema, generate_string_schema,
    make_schema_optional,
};
use crate::{Configurable, GenerateError};

// Numbers.
macro_rules! impl_configurable_numeric {
	($($ty:ty),+) => {
		$(
			impl Configurable for $ty {
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

impl<K, V> Configurable for BTreeMap<K, V>
where
    K: ConfigurableString + Ord,
    V: Configurable,
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

impl<K, V> Configurable for HashMap<K, V>
where
    K: ConfigurableString + std::hash::Hash + Eq,
    V: Configurable,
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

impl Configurable for Cow<'static, str> {
    fn generate_schema(_gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        Ok(generate_string_schema())
    }
}

impl Configurable for bool {
    fn generate_schema(_gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        Ok(generate_bool_schema())
    }
}

impl<T> Configurable for Option<T>
where
    T: Configurable,
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
    T: Configurable,
{
    fn generate_schema(gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        generate_array_schema::<T>(gen)
    }
}

impl Configurable for Duration {
    fn generate_schema(_gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        Ok(SchemaObject {
            instance_type: Some(InstanceType::String.into()),
            metadata: Some(
                Metadata {
                    examples: vec![Value::String("1m".to_string())],
                    ..Default::default()
                }
                .into(),
            ),
            ..Default::default()
        })
    }
}

impl Configurable for PathBuf {
    fn generate_schema(_: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        let mut schema = generate_string_schema();
        let metadata = schema.metadata();

        metadata.description = Some("file path".to_string());

        Ok(generate_string_schema())
    }
}

// Additional types that do not map directly to scalars.
impl Configurable for SocketAddr {
    fn generate_schema(_: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        // TODO: We don't need anything other than a string schema to (de)serialize a `SocketAddr`,
        // but we eventually should have validation since the format for the possible permutations
        // is well-known and can be easily codified.
        let mut schema = generate_string_schema();
        let metadata = schema.metadata();
        metadata.description = Some("An internet socket address, either IPv4 or IPv6.".to_string());
        metadata.examples = vec![Value::String("127.0.0.1:8080".to_owned())];
        schema.format = Some("ip-address".to_string());

        Ok(schema)
    }
}
