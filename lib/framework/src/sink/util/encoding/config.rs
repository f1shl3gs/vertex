use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

use event::log::path_iter::{PathComponent, PathIter};
use serde::de::{DeserializeOwned, Error, IntoDeserializer, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

use super::{EncodingConfiguration, TimestampFormat};

// Deduplicate codes
#[inline]
fn skip_serializing_if_default<E: Default + PartialEq>(e: &E) -> bool {
    e == &E::default()
}

/// A structure to wrap encodings and enforce field privacy
///
/// This structure **does not** assume that there is a default format. Consider
/// `EncodingConfigWithDefault<E>` instead if `E: Default`
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EncodingConfig<E> {
    pub codec: E,
    #[serde(default, skip_serializing_if = "skip_serializing_if_default")]
    pub schema: Option<String>,
    #[serde(default, skip_serializing_if = "skip_serializing_if_default")]
    pub only_fields: Option<Vec<Vec<PathComponent<'static>>>>,
    #[serde(default, skip_serializing_if = "skip_serializing_if_default")]
    pub except_fields: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "skip_serializing_if_default")]
    pub timestamp_format: Option<TimestampFormat>,
}

impl<E> EncodingConfiguration for EncodingConfig<E> {
    type Codec = E;

    fn codec(&self) -> &Self::Codec {
        &self.codec
    }

    fn schema(&self) -> &Option<String> {
        &self.schema
    }

    fn only_fields(&self) -> &Option<Vec<Vec<PathComponent>>> {
        &self.only_fields
    }

    fn except_fields(&self) -> &Option<Vec<String>> {
        &self.except_fields
    }

    fn timestamp_format(&self) -> &Option<TimestampFormat> {
        &self.timestamp_format
    }
}

impl<E> From<E> for EncodingConfig<E> {
    fn from(codec: E) -> Self {
        Self {
            codec,
            schema: Default::default(),
            only_fields: Default::default(),
            except_fields: Default::default(),
            timestamp_format: Default::default(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
pub struct Inner<E> {
    codec: E,
    #[serde(default)]
    schema: Option<String>,
    #[serde(default)]
    only_fields: Option<Vec<String>>,
    #[serde(default)]
    except_fields: Option<Vec<String>>,
    #[serde(default)]
    timestamp_format: Option<TimestampFormat>,
}

impl<'de, E> Deserialize<'de> for EncodingConfig<E>
where
    E: DeserializeOwned + Serialize + Debug + Clone + PartialEq + Eq,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // This is a Visitor that forwards string types to T's `FromStr` impl and forwards
        // map types to T's `Deserialize` impl. The `PhantomData` is to keep the compiler
        // from complaining about T being an unused generic type parameter. We need T in
        // order to known the Value type for the Visitor impl.
        struct StringOrStruct<T: DeserializeOwned + Serialize + Debug + Eq + PartialEq + Clone>(
            PhantomData<fn() -> T>,
        );

        impl<'de, T> Visitor<'de> for StringOrStruct<T>
        where
            T: DeserializeOwned + Serialize + Debug + Eq + PartialEq + Clone,
        {
            type Value = Inner<T>;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("string or map")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(Self::Value {
                    codec: T::deserialize(v.into_deserializer())?,
                    schema: Default::default(),
                    only_fields: Default::default(),
                    except_fields: Default::default(),
                    timestamp_format: Default::default(),
                })
            }

            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                // `MapAccessDeserializer` is a wrapper that turns a `MapAccess` into a `Deserializer`,
                // allowing it to be used as the input to T's `Deserializer` implementation. T then
                // deserializes itself using the entries from the map visitor.
                Deserialize::deserialize(serde::de::value::MapAccessDeserializer::new(map))
            }
        }

        let inner = deserializer.deserialize_any(StringOrStruct::<E>(PhantomData))?;
        let concrete = Self {
            codec: inner.codec,
            schema: inner.schema,

            only_fields: inner.only_fields.map(|fields| {
                fields
                    .iter()
                    .map(|only| {
                        PathIter::new(only)
                            .map(|component| component.into_static())
                            .collect()
                    })
                    .collect()
            }),
            except_fields: inner.except_fields,
            timestamp_format: inner.timestamp_format,
        };

        concrete.validate().map_err(serde::de::Error::custom)?;
        Ok(concrete)
    }
}
