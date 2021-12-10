use chrono::{DateTime, Local, ParseError, TimeZone as _, Utc};
use chrono_tz::Tz;
use derivative::Derivative;

#[derive(Clone, Copy, Debug, Derivative, Eq, PartialEq)]
#[derivative(Default)]
pub enum TimeZone {
    #[derivative(Default)]
    Local,
    Named(Tz),
}

/// This is a wrapper trait to allow `TimeZone` types to be passed generically
impl TimeZone {
    pub fn datetime_from_str(&self, s: &str, format: &str) -> Result<DateTime<Utc>, ParseError> {
        match self {
            Self::Local => Local.datetime_from_str(s, format).map(datetime_to_utc),
            Self::Named(tz) => tz.datetime_from_str(s, format).map(datetime_to_utc),
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
pub(super) fn datetime_to_utc<TZ: chrono::TimeZone>(ts: DateTime<TZ>) -> DateTime<Utc> {
    Utc.timestamp(ts.timestamp(), ts.timestamp_subsec_micros())
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

        let tz = w.tz;
        println!("{:?}", w.tz.name());
    }
}
