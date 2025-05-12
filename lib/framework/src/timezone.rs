use chrono::format::{Parsed, StrftimeItems, parse};
use chrono::{DateTime, Local, ParseError, Utc};
use chrono_tz::Tz;
use configurable::Configurable;
use configurable::schema::{SchemaGenerator, SchemaObject};

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
        let mut parsed = Parsed::new();
        parse(&mut parsed, s, StrftimeItems::new(format))?;

        match self {
            TimeZone::Local => parsed
                .to_datetime_with_timezone(&Local)
                .map(|dt| datetime_to_utc(&dt)),
            TimeZone::Named(tz) => parsed
                .to_datetime_with_timezone(tz)
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
    DateTime::<Utc>::from_timestamp(ts.timestamp(), ts.timestamp_subsec_nanos())
        .expect("convert to utc timestamp failed")
}

impl Configurable for TimeZone {
    fn reference() -> Option<&'static str> {
        Some(std::any::type_name::<Self>())
    }

    fn generate_schema(generator: &mut SchemaGenerator) -> SchemaObject {
        let mut local = SchemaObject::const_value("local");
        local.metadata.description = Some("System local timezone.");

        let mut tz = generator.subschema_for::<Tz>();
        tz.metadata.description = Some(
            r#"Must be a valid name in the [TZ database]: https://en.wikipedia.org/wiki/List_of_tz_database_time_zones"#,
        );

        SchemaObject::one_of(
            vec![local, tz],
            Some(
                r#"This can refer to any valid timezone as defined in the [TZ database]: https://en.wikipedia.org/wiki/List_of_tz_database_time_zones, or "local" which refers to the system local timezone."#,
            ),
        )
    }
}

pub mod ser_de {
    use std::fmt::Formatter;

    use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

    use super::*;

    struct TimeZoneVisitor;

    impl de::Visitor<'_> for TimeZoneVisitor {
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
