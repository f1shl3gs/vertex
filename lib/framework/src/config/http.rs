pub mod uri {
    use std::fmt;

    use http::Uri;
    use serde::de::{Unexpected, Visitor};
    use serde::{Deserializer, Serializer, de};

    #[inline]
    pub fn serialize<S: Serializer>(uri: &Uri, ser: S) -> Result<S::Ok, S::Error> {
        ser.collect_str(&uri)
    }

    struct UriVisitor;

    impl<'de> Visitor<'de> for UriVisitor {
        type Value = Uri;

        #[inline]
        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("uri")
        }

        fn visit_str<E: de::Error>(self, val: &str) -> Result<Self::Value, E> {
            val.parse()
                .map_err(|_| de::Error::invalid_value(Unexpected::Str(val), &self))
        }

        fn visit_string<E: de::Error>(self, val: String) -> Result<Self::Value, E> {
            val.try_into().map_err(de::Error::custom)
        }

        #[inline]
        fn visit_some<D: Deserializer<'de>>(
            self,
            deserializer: D,
        ) -> Result<Self::Value, D::Error> {
            deserializer.deserialize_str(self)
        }
    }

    /// Implementation detail.
    #[inline]
    pub fn deserialize<'de, D>(de: D) -> Result<Uri, D::Error>
    where
        D: Deserializer<'de>,
    {
        de.deserialize_str(UriVisitor)
    }
}
