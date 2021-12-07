use std::fmt::Formatter;
use serde::de::{Error, MapAccess};
use serde::Deserializer;

#[derive(Debug, Eq, PartialEq)]
enum Compression {
    None,
    Gzip(i32)
}

impl<'de> serde::de::Deserialize<'de> for Compression {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        struct StringOrMap;

        impl<'de> serde::de::Visitor<'de> for StringOrMap {
            type Value = Compression;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("string or map")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> where E: Error {
                match v {
                    "none" => Ok(Compression::None),
                    "gzip" => Ok(Compression::Gzip(6)),
                    _ => Err(serde::de::Error::invalid_value(
                        serde::de::Unexpected::Str(v),
                        &r#"none or gzip"#,
                    ))
                }
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error> where A: MapAccess<'de> {
                while let Some(key) = map.next_key()? {
                    match key {
                        "algorithm" | "alg" => println!("alg"),
                        "level" => println!("level"),
                        _ => return Err(serde::de::Error::unknown_field(
                            key,
                            &["alg", "algorithm", "level"]
                        ))
                    }
                }

                todo!()
            }
        }

        deserializer.deserialize_any(StringOrMap)
    }
}

#[test]
fn deserialize() {
    let want = Compression::Gzip(2);
    let compression = serde_yaml::from_str::<Compression>("algorithm: gzip\nlevel: 2").unwrap();
    assert_eq!(want, compression)
}