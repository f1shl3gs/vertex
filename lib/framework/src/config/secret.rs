use std::fmt::Formatter;
use std::ops::Deref;

use configurable::Configurable;
use configurable::schema::{SchemaGenerator, SchemaObject};
use serde::de::Error;
use serde::{Deserialize, Serialize, Serializer};

/// A simple wrapper for sensitive strings containing credentials
///
/// This is very necessary, since config will be exposed
#[derive(Clone, PartialEq, Eq)]
pub struct SecretString(String);

impl From<String> for SecretString {
    fn from(value: String) -> Self {
        SecretString(value)
    }
}

impl From<&str> for SecretString {
    fn from(value: &str) -> Self {
        SecretString(value.to_string())
    }
}

impl From<SecretString> for String {
    fn from(s: SecretString) -> Self {
        s.0
    }
}

impl Configurable for SecretString {
    fn generate_schema(generator: &mut SchemaGenerator) -> SchemaObject {
        generator.subschema_for::<String>()
    }
}

impl std::fmt::Debug for SecretString {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("******")
    }
}

impl std::fmt::Display for SecretString {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("******")
    }
}

impl Deref for SecretString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<'de> Deserialize<'de> for SecretString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct SecretStringVisitor;

        impl serde::de::Visitor<'_> for SecretStringVisitor {
            type Value = SecretString;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> {
                Ok(SecretString(v.to_owned()))
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(SecretString(v))
            }
        }

        deserializer.deserialize_str(SecretStringVisitor)
    }
}

impl Serialize for SecretString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str("******")
    }
}

impl SecretString {
    #[inline]
    pub fn inner(&self) -> &str {
        self.0.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialization() {
        let secret = SecretString("foobar".to_string());
        let text = serde_json::to_string(&secret).unwrap();
        assert_eq!(text, "\"******\"");
    }

    #[test]
    fn deserialize() {
        let secret = "\"foobar\"";
        let s = serde_json::from_str::<SecretString>(secret).unwrap();
        assert_eq!(s.deref(), "foobar");
    }
}
