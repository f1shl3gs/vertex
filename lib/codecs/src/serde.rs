/// Default value for the UTF-8 lossy option.
pub fn default_lossy() -> bool {
    true
}

/// Answers "Is it possible to skip serializing this value, because it's the
/// default?"
#[inline]
pub fn skip_serializing_if_default<T: Default + PartialEq>(e: &T) -> bool {
    e == &T::default()
}

/// Handling of ASCII characters in `u8` fields via `serde`s `with` attribute.
pub mod ascii_char {
    use serde::{de, Deserialize, Deserializer, Serializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u8, D::Error>
    where
        D: Deserializer<'de>,
    {
        let character = char::deserialize(deserializer)?;
        if character.is_ascii() {
            Ok(character as u8)
        } else {
            Err(de::Error::custom(format!(
                "invalid character: {character}, expected character in ASCII range"
            )))
        }
    }

    pub fn serialize<S>(character: &u8, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_char(*character as char)
    }
}
