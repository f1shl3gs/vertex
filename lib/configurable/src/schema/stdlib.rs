use std::collections::{BTreeMap, HashMap};
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::time::Duration;

use serde_json::Value;

use crate::Configurable;
use crate::configurable::ConfigurableString;
use crate::schema::{
    ArrayValidation, InstanceType, Metadata, NumberValidation, ObjectValidation, SchemaGenerator,
    SchemaObject, SingleOrVec, SubschemaValidation, assert_string_schema_for_map,
};

macro_rules! impl_integer {
    ($($typ:ty),+) => {
        $(
            impl Configurable for $typ {
                fn generate_schema(_generator: &mut SchemaGenerator) -> SchemaObject {
                    SchemaObject {
                        instance_type: Some(InstanceType::Integer.into()),
                        number: Some(Box::new(NumberValidation {
                            minimum: Some(<$typ>::MIN as f64),
                            maximum: Some(<$typ>::MAX as f64),
                        })),
                        ..Default::default()
                    }
                }
            }
        )+
    };
}

impl_integer!(u8, u16, u32, u64, usize, i8, i16, i32, i64, isize);

macro_rules! impl_number {
    ($($typ:ty),+) => {
        $(
            impl Configurable for $typ {
                fn generate_schema(_generator: &mut SchemaGenerator) -> SchemaObject {
                    SchemaObject {
                        instance_type: Some(InstanceType::Number.into()),
                        number: Some(Box::new(NumberValidation {
                            minimum: Some(<$typ>::MIN as f64),
                            maximum: Some(<$typ>::MAX as f64),
                        })),
                        ..Default::default()
                    }
                }
            }
        )+
    };
}
impl_number!(f32, f64);

macro_rules! impl_nonzero {
    ($($typ:ty => $inner:ty),+) => {
        $(
            impl Configurable for $typ {
                fn generate_schema(_generator: &mut SchemaGenerator) -> SchemaObject {
                    SchemaObject {
                        instance_type: Some(InstanceType::Integer.into()),
                        number: Some(Box::new(NumberValidation {
                            minimum: Some(1.0),
                            maximum: Some(<$inner>::MAX as f64),
                        })),
                        ..Default::default()
                    }
                }
            }
        )+
    };
}

impl_nonzero!(
    std::num::NonZeroU8 => u8,
    std::num::NonZeroU16 => u16,
    std::num::NonZeroU32 => u32,
    std::num::NonZeroU64 => u64,
    std::num::NonZeroUsize => usize,
    std::num::NonZeroI8 => i8,
    std::num::NonZeroI16 => i16,
    std::num::NonZeroI32 => i32,
    std::num::NonZeroI64 => i64,
    std::num::NonZeroIsize => isize
);

impl<K, V> Configurable for BTreeMap<K, V>
where
    K: ConfigurableString + Ord,
    V: Configurable,
{
    fn generate_schema(generator: &mut SchemaGenerator) -> SchemaObject {
        // Make sure our key type is _truly_ a string schema.
        assert_string_schema_for_map::<K, Self>(generator).expect("key must be string like");

        SchemaObject {
            instance_type: Some(InstanceType::Object.into()),
            object: Some(Box::new(ObjectValidation {
                additional_properties: Some(Box::new(generator.subschema_for::<V>())),
                ..Default::default()
            })),
            ..Default::default()
        }
    }
}

impl<K, V> Configurable for HashMap<K, V>
where
    K: ConfigurableString + std::hash::Hash + Eq,
    V: Configurable,
{
    fn generate_schema(generator: &mut SchemaGenerator) -> SchemaObject {
        // Make sure our key type is _truly_ a string schema.
        assert_string_schema_for_map::<K, Self>(generator).expect("key must be string like");

        SchemaObject {
            instance_type: Some(InstanceType::Object.into()),
            object: Some(Box::new(ObjectValidation {
                additional_properties: Some(Box::new(generator.subschema_for::<V>())),
                ..Default::default()
            })),
            ..Default::default()
        }
    }
}

impl Configurable for String {
    fn generate_schema(_gen: &mut SchemaGenerator) -> SchemaObject {
        SchemaObject {
            instance_type: Some(InstanceType::String.into()),
            ..Default::default()
        }
    }
}

impl Configurable for bool {
    fn generate_schema(_gen: &mut SchemaGenerator) -> SchemaObject {
        SchemaObject {
            instance_type: Some(InstanceType::Boolean.into()),
            ..Default::default()
        }
    }
}

impl<T: Configurable> Configurable for Option<T> {
    fn reference() -> Option<&'static str> {
        match T::reference() {
            Some(_) => Some(std::any::type_name::<Self>()),
            None => None,
        }
    }

    fn generate_schema(generator: &mut SchemaGenerator) -> SchemaObject {
        let mut schema = T::generate_schema(generator);

        match schema.instance_type.as_mut() {
            None => match schema.subschemas.as_mut() {
                None => panic!("invalid option field type"),
                Some(subschemas) => {
                    if let Some(any_of) = subschemas.any_of.as_mut() {
                        any_of.push(SchemaObject::null());
                    } else if let Some(one_of) = subschemas.one_of.as_mut() {
                        one_of.push(SchemaObject::null());
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

                        subschemas.one_of = Some(vec![SchemaObject::null(), new_all_of_schema]);
                    } else {
                        panic!("invalid option field type")
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

        schema
    }
}

// Array
impl<T> Configurable for Vec<T>
where
    T: Configurable,
{
    fn generate_schema(generator: &mut SchemaGenerator) -> SchemaObject {
        // Generate the actual schema for the element type `T`.
        let element_schema = generator.subschema_for::<T>();

        SchemaObject {
            instance_type: Some(InstanceType::Array.into()),
            array: Some(Box::new(ArrayValidation {
                items: Some(SingleOrVec::Single(Box::new(element_schema))),
            })),
            ..Default::default()
        }
    }
}

impl Configurable for Duration {
    fn generate_schema(_gen: &mut SchemaGenerator) -> SchemaObject {
        SchemaObject {
            instance_type: Some(InstanceType::String.into()),
            metadata: Metadata {
                examples: vec![Value::String("1m".to_string())],
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

impl Configurable for PathBuf {
    fn generate_schema(_: &mut SchemaGenerator) -> SchemaObject {
        SchemaObject {
            metadata: Metadata {
                description: Some("file path"),
                ..Default::default()
            },
            instance_type: Some(InstanceType::String.into()),
            ..Default::default()
        }
    }
}

// Additional types that do not map directly to scalars.
impl Configurable for SocketAddr {
    fn generate_schema(_: &mut SchemaGenerator) -> SchemaObject {
        SchemaObject {
            metadata: Metadata {
                description: Some("An internet socket address, either IPv4 or IPv6."),
                examples: vec![Value::String("127.0.0.1:8080".to_owned())],
                ..Default::default()
            },
            instance_type: Some(InstanceType::String.into()),
            ..Default::default()
        }
    }
}

impl Configurable for IpAddr {
    fn generate_schema(_gen: &mut SchemaGenerator) -> SchemaObject {
        SchemaObject {
            metadata: Metadata {
                description: Some("IPv4 or IPv6 Address"),
                examples: vec![Value::String("192.168.0.1".to_owned())],
                ..Default::default()
            },
            ..Default::default()
        }
    }
}
