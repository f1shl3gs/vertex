use chrono::{DateTime, Local, ParseError, TimeZone as _, Utc};
use chrono_tz::Tz;
use configurable::schema::{
    generate_const_string_schema, generate_one_of_schema, get_or_generate_schema, SchemaGenerator,
    SchemaObject,
};
use configurable::{Configurable, GenerateError};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub enum TimeZone {
    /// System local timezone.
    #[default]
    Local,

    /// A named timezone.
    ///
    /// Must be a valid name in the [TZ database][tzdb].
    ///
    /// [tzdb]: https://en.wikipedia.org/wiki/List_of_tz_database_time_zones
    Named(Tz),
}

/// This is a wrapper trait to allow `TimeZone` types to be passed generically
impl TimeZone {
    pub fn datetime_from_str(&self, s: &str, format: &str) -> Result<DateTime<Utc>, ParseError> {
        match self {
            Self::Local => Local
                .datetime_from_str(s, format)
                .map(|dt| datetime_to_utc(&dt)),
            Self::Named(tz) => tz
                .datetime_from_str(s, format)
                .map(|dt| datetime_to_utc(&dt)),
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "" | "local" => Some(Self::Local),
            _ => s.parse::<Tz>().ok().map(Self::Named),
        }
    }
}

/// Convert a timestamp with a non-UTC time zone into UTC
pub fn datetime_to_utc<TZ: chrono::TimeZone>(ts: &DateTime<TZ>) -> DateTime<Utc> {
    Utc.timestamp_nanos(ts.timestamp_nanos())
}

impl Configurable for TimeZone {
    fn reference() -> Option<&'static str> {
        Some(std::any::type_name::<Self>())
    }

    fn generate_schema(gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        let mut local_schema = generate_const_string_schema("local".to_string());
        let metadata = local_schema.metadata();
        metadata.description = Some("System local timezone.".to_string());

        let mut tz_schema = get_or_generate_schema::<Tz>(gen)?;
        let metadata = tz_schema.metadata();
        metadata.title = Some("A named timezone".to_string());
        metadata.description = Some(r#"Must be a valid name in the [TZ database]: https://en.wikipedia.org/wiki/List_of_tz_database_time_zones"#.to_string());

        let mut schema = generate_one_of_schema(&[local_schema, tz_schema]);
        let metadata = schema.metadata();
        metadata.title = Some("Timezone reference.".to_string());
        metadata.description = Some(r#"This can refer to any valid timezone as defined in the [TZ database]: https://en.wikipedia.org/wiki/List_of_tz_database_time_zones, or "local" which refers to the system local timezone."#.to_string());

        Ok(schema)
    }
}

pub mod ser_de {
    use super::*;
    use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
    use std::fmt::Formatter;

    struct TimeZoneVisitor;

    impl<'de> de::Visitor<'de> for TimeZoneVisitor {
        type Value = TimeZone;

        fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
            write!(formatter, "a time zone name")
        }

        fn visit_str<E: de::Error>(self, s: &str) -> Result<Self::Value, E> {
            match TimeZone::parse(s) {
                Some(tz) => Ok(tz),
                None => Err(de::Error::custom("No such time zone")),
            }
        }
    }

    impl Serialize for TimeZone {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            match self {
                Self::Local => serializer.serialize_str("local"),
                Self::Named(tz) => serializer.serialize_str(tz.name()),
            }
        }
    }

    impl<'de> Deserialize<'de> for TimeZone {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer.deserialize_str(TimeZoneVisitor)
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono_tz::Tz;
    use serde::{Deserialize, Serialize};

    #[test]
    fn deserialize() {
        #[derive(Deserialize, Serialize)]
        struct TzWrapper {
            tz: Tz,
        }

        let input = r#"
tz: CET
        "#;

        let w: TzWrapper = serde_yaml::from_str(input).unwrap();
        assert_eq!(w.tz.name(), "CET");
    }
}
