use std::borrow::Cow;
use std::num::ParseFloatError;

use serde::{Deserialize, Deserializer, Serializer};
use snafu::{ResultExt, Snafu};

// ICE Sizes, kibis of bits
const BYTE: usize = 1;
const KIBYTE: usize = 1 << 10;
const MIBYTE: usize = 1 << (2 * 10);
const GIBYTE: usize = 1 << (3 * 10);
const TIBYTE: usize = 1 << (4 * 10);
const PIBYTE: usize = 1 << (5 * 10);
const EIBYTE: usize = 1 << (6 * 10);

// SI Sizes
const IBYTE: usize = 1;
const KBYTE: usize = IBYTE * 1000;
const MBYTE: usize = KBYTE * 1000;
const GBYTE: usize = MBYTE * 1000;
const TBYTE: usize = GBYTE * 1000;
const PBYTE: usize = TBYTE * 1000;
const EBYTE: usize = PBYTE * 1000;

#[derive(Debug, Snafu)]
pub enum ParseBytesError {
    #[snafu(display("Parse float part failed, {}", source))]
    ParseFloatFailed { source: ParseFloatError },
    #[snafu(display("Unknown unit \"{}\" found", unit))]
    UnknownUnit { unit: String },
    #[snafu(display("Too large: {}", input))]
    TooLarge { input: String },
}

/// bytes produces a human readable representation of an SI size
///
/// See also: parse_bytes
///
/// Bytes(82854982) -> 83 MB
pub fn bytes(s: usize) -> String {
    humanate_bytes(s, 1000.0, ["B", "kB", "MB", "GB", "TB", "PB", "EB"])
}

/// ibytes produces a human readable representation of an IEC size.
///
/// IBytes(82854982) -> 79 MiB
pub fn ibytes(s: usize) -> String {
    humanate_bytes(s, 1024.0, ["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB"])
}

/// parse_bytes parses a string representation of bytes into the number of bytes it represents
///
/// parse_bytes("42 MB") -> Ok(42000000)
/// parse_bytes("42 mib") -> Ok(44040192)
pub fn parse_bytes(s: &str) -> Result<usize, ParseBytesError> {
    let mut last_digit = 0;
    let mut has_comma = false;

    for c in s.chars() {
        if !(c.is_digit(10) || c == '.' || c == ',') {
            break;
        }

        if c == ',' {
            has_comma = true;
        }

        last_digit += 1;
    }

    let num = &s[..last_digit];
    let mut tn = num.to_string();
    if has_comma {
        tn = num.replace(",", "");
    }

    let f = tn.parse::<f64>().context(ParseFloatFailed)?;
    let extra = &s[last_digit..];
    let extra = extra.trim().to_lowercase();

    let m = match extra.as_str() {
        "b" | "" => BYTE,
        "kib" | "ki" => KIBYTE,
        "kb" | "k" => KBYTE,
        "mib" | "mi" => MIBYTE,
        "mb" | "m" => MBYTE,
        "gib" | "gi" => GIBYTE,
        "gb" | "g" => GBYTE,
        "tib" | "ti" => TIBYTE,
        "tb" | "t" => TBYTE,
        "pib" | "pi" => PIBYTE,
        "pb" | "p" => PBYTE,
        "eib" | "ei" => EIBYTE,
        "eb" | "e" => EBYTE,
        _ => {
            return Err(ParseBytesError::UnknownUnit {
                unit: extra.clone(),
            });
        }
    };

    Ok((f * m as f64) as usize)
}

#[inline]
fn logn(n: f64, b: f64) -> f64 {
    n.log2() / b.log2()
}

fn humanate_bytes(s: usize, base: f64, sizes: [&str; 7]) -> String {
    if s < 10 {
        return format!("{} B", s);
    }

    let e = logn(s as f64, base);
    let e = e.floor();
    let suffix = sizes[e as usize];
    let val = s as f64 / base.powf(e) * 10.0 + 0.5;
    let val = val.floor() / 10.0;

    format!("{} {}", val, suffix)
}

pub fn deserialize_bytes<'de, D: Deserializer<'de>>(deserializer: D) -> Result<usize, D::Error> {
    let s: Cow<str> = serde::__private::de::borrow_cow_str(deserializer)?;
    parse_bytes(&s).map_err(serde::de::Error::custom)
}

pub fn serialize_bytes<S: Serializer>(u: &usize, s: S) -> Result<S::Ok, S::Error> {
    let b = bytes(*u);
    s.serialize_str(&b)
}

pub fn deserialize_bytes_option<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<usize>, D::Error> {
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s {
        None => Ok(None),
        Some(s) => parse_bytes(&s).map_err(serde::de::Error::custom),
    }
}

pub fn serialize_bytes_option<S: Serializer>(u: &Option<usize>, s: S) -> Result<S::Ok, S::Error> {
    match u {
        Some(v) => s.serialize_str(bytes(*v).as_str()),
        None => s.serialize_none(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bytes() {
        let tests = [
            ("42MB", 42000000),
            ("128MB", 128 * 1024 * 1024),
            ("128M", 128 * 1024 * 1024),
            ("128.0MB", 128 * 1024 * 1024),
            ("128.0m", 128 * 1024 * 1024),
            ("128.0 MB", 128 * 1024 * 1024),
        ];

        for (input, want) in tests {
            let value = parse_bytes(input).unwrap();
        }
    }
}
