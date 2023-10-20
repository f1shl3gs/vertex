#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::num::{ParseFloatError, ParseIntError};

use bytes::Bytes;
use chrono::{DateTime, Utc};
use event::log::OwnedTargetPath;
use framework::timezone::{datetime_to_utc, TimeZone};
use thiserror::Error;

/// `Conversion` is a place-holder for a type conversion operation, to convert
/// from a plain `Bytes` into another type. The inner type of every `Value`
/// variant is represented here.
#[derive(Clone, Debug)]
pub enum Conversion {
    Bytes,
    Integer,
    Float,
    Boolean,
    Timestamp(TimeZone),
    TimestampFmt(String, TimeZone),
    TimestampTzFmt(String),
}

#[derive(Debug, Eq, PartialEq, Error)]
pub enum Error {
    #[error("Unknown conversion name {0:?}")]
    UnknownConversion(String),

    #[error("Invalid boolean value {0:?}")]
    BooleanParse(String),
    #[error("Invalid integer {s}: {err}")]
    IntegerParse { s: String, err: ParseIntError },
    #[error("Invalid float {s}: {err}")]
    FloatParse { s: String, err: ParseFloatError },
    #[error("Invalid timestamp {s:?}: {err}")]
    TimestampParse { s: String, err: chrono::ParseError },
    #[error("No matching timestamp format found for {0:?}")]
    AutoTimestampParse(String),
}

/// Helper function to parse a mapping of conversion descriptions into actual
/// Conversion values.
pub fn parse_conversion_map(
    types: &HashMap<OwnedTargetPath, String>,
    tz: TimeZone,
) -> Result<HashMap<OwnedTargetPath, Conversion>, Error> {
    types
        .iter()
        .map(|(field, typename)| Conversion::parse(typename, tz).map(|conv| (field.clone(), conv)))
        .collect()
}

impl Conversion {
    /// Convert the string into a type conversion. The following conversion names are
    /// supported:
    ///
    /// * `"bytes"`, `"str"` or `"string"` => As-is (no conversion)
    /// * `"int"` or `"integer"` => Signed integer
    /// * `"float"` => Floating point number
    /// * `"bool"` or `"boolean"` => Boolean
    /// * `"timestamp"` => Timestamp, guessed using a set of formats
    pub fn parse(s: impl AsRef<str>, tz: TimeZone) -> Result<Conversion, Error> {
        let s = s.as_ref();
        let mut split = s.splitn(2, '|').map(str::trim);
        match (split.next(), split.next()) {
            (Some("bytes" | "str" | "string"), None) => Ok(Self::Bytes),
            (Some("integer" | "int"), None) => Ok(Self::Integer),
            (Some("float"), None) => Ok(Self::Float),
            (Some("bool" | "boolean"), None) => Ok(Self::Boolean),
            (Some("timestamp"), None) => Ok(Self::Timestamp(tz)),
            (Some("timestamp"), Some(fmt)) => {
                // DateTime<Utc> can only convert timestamps without time zones, and
                // DateTime<FixedOffset> can only convert with tone zones, so this has to
                // distinguish between the two types of formats.
                if format_has_zone(fmt) {
                    Ok(Self::TimestampTzFmt(fmt.into()))
                } else {
                    Ok(Self::TimestampFmt(fmt.into(), tz))
                }
            }
            _ => Err(Error::UnknownConversion(s.into())),
        }
    }

    /// Use this `conversion` variant to turn the given `bytes` into a new `T`.
    pub fn convert<T>(&self, bytes: Bytes) -> Result<T, Error>
    where
        T: From<Bytes> + From<i64> + From<f64> + From<bool> + From<DateTime<Utc>>,
    {
        Ok(match self {
            Self::Bytes => bytes.into(),
            Self::Integer => {
                let s = String::from_utf8_lossy(&bytes);
                s.parse::<i64>()
                    .map_err(|err| Error::IntegerParse {
                        s: s.to_string(),
                        err,
                    })?
                    .into()
            }
            Self::Float => {
                let s = String::from_utf8_lossy(&bytes);
                s.parse::<f64>()
                    .map_err(|err| Error::FloatParse {
                        s: s.to_string(),
                        err,
                    })?
                    .into()
            }
            Self::Boolean => parse_bool(&String::from_utf8_lossy(&bytes))?.into(),
            Self::Timestamp(tz) => parse_timestamp(*tz, &String::from_utf8_lossy(&bytes))?.into(),
            Self::TimestampFmt(format, tz) => {
                let s = String::from_utf8_lossy(&bytes);
                let dt = tz
                    .datetime_from_str(&s, format)
                    .map_err(|err| Error::TimestampParse {
                        s: s.to_string(),
                        err,
                    })?;

                datetime_to_utc(&dt).into()
            }
            Self::TimestampTzFmt(format) => {
                let s = String::from_utf8_lossy(&bytes);
                let dt =
                    DateTime::parse_from_str(&s, format).map_err(|err| Error::TimestampParse {
                        s: s.to_string(),
                        err,
                    })?;

                datetime_to_utc(&dt).into()
            }
        })
    }
}

/// Parse a string into a native `bool`. The built in `bool::from_str`
/// only handles two cases, `"true"` and `"false"`. we want to be able
/// to convert from a more diverse set of strings. In particular, the
/// following set of source strings are allowed:
///
/// * `"true"`, `"t"`, `"yes"`, `"y"`, `"on"` (all case-insensitive), and
/// non-zero integers all convert to `true`.
///
/// * `"false"`, `"f"`, `"no"`, `"n"`, `"off"` (all case-insensitive), and `"0"`
/// all convert to `false`.
fn parse_bool(s: &str) -> Result<bool, Error> {
    match s {
        "true" | "t" | "yes" | "y" | "on" => Ok(true),
        "false" | "f" | "no" | "n" | "off" => Ok(false),
        _ => {
            if let Ok(n) = s.parse::<i64>() {
                Ok(n != 0)
            } else {
                // Do the case conversion only if simple matches fail,
                // since this operation can be expensive.
                match s.to_lowercase().as_str() {
                    "true" | "t" | "yes" | "y" | "on" => Ok(true),
                    "false" | "f" | "no" | "n" | "off" => Ok(false),
                    _ => Err(Error::BooleanParse(s.into())),
                }
            }
        }
    }
}

/// Does the format specifier have a time zone option?
fn format_has_zone(fmt: &str) -> bool {
    fmt.contains("%Z")
        || fmt.contains("%z")
        || fmt.contains("%:z")
        || fmt.contains("%#z")
        || fmt.contains("%+")
}

/// The list of allowed "automatic" timestamp formats with assumed local time zone
const TIMESTAMP_LOCAL_FORMATS: &[&str] = &[
    "%F %T",           // YYYY-MM-DD HH:MM:SS
    "%v %T",           // DD-Mmm-YYYY HH:MM:SS
    "%FT%T",           // ISO 8601 / RFC 3339 without TZ
    "%m/%d/%Y:%T",     // ???
    "%a, %d %b %Y %T", // RFC 822/2822 without TZ
    "%a %d %b %T %Y",  // `date` command output without TZ
    "%A %d %B %T %Y",  // `date` command output without TZ, long names
    "%a %b %e %T %Y",  // ctime format
];

/// The list of allowed "automatic" timestamp formats for UTC
const TIMESTAMP_UTC_FORMATS: &[&str] = &[
    "%s",     // UNIX timestamp
    "%FT%TZ", // ISO 8601 / RFC 3339 UTC
];

/// The list of allowed "automatic" timestamp formats with time zones
const TIMESTAMP_TZ_FORMATS: &[&str] = &[
    "%+",                 // ISO 8601 / RFC 3339
    "%a %d %b %T %Z %Y",  // `date` command output
    "%a %d %b %T %z %Y",  // `date` command output, numeric TZ
    "%a %d %b %T %#z %Y", // `date` command output, numeric TZ
    "%d/%b/%Y:%T %z",     // Common Log
];

/// Parse a string into a timestamp using one of a set of formats
fn parse_timestamp(tz: TimeZone, s: &str) -> Result<DateTime<Utc>, Error> {
    for format in TIMESTAMP_LOCAL_FORMATS {
        if let Ok(result) = tz.datetime_from_str(s, format) {
            return Ok(result);
        }
    }

    for format in TIMESTAMP_UTC_FORMATS {
        if let Ok(result) = DateTime::parse_from_str(s, format) {
            return Ok(datetime_to_utc(&result));
        }
    }

    if let Ok(result) = DateTime::parse_from_rfc3339(s) {
        return Ok(datetime_to_utc(&result));
    }

    if let Ok(result) = DateTime::parse_from_rfc2822(s) {
        return Ok(datetime_to_utc(&result));
    }

    for format in TIMESTAMP_TZ_FORMATS {
        if let Ok(result) = DateTime::parse_from_str(s, format) {
            return Ok(datetime_to_utc(&result));
        }
    }

    Err(Error::AutoTimestampParse(s.into()))
}
