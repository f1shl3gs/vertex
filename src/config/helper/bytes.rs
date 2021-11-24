use std::fmt::format;
use std::num::ParseFloatError;

use snafu::{Snafu, ResultExt};


// ICE Sizes, kibis of bits
const BYTE: usize = 1 << (0 * 10);
const KIBYTE: usize = 1 << (1 * 10);
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
pub fn bytes(s: u64) -> String {
    humanate_bytes(s, 1000.0, ["B", "kB", "MB", "GB", "TB", "PB", "EB"])
}

/// ibytes produces a human readable representation of an IEC size.
///
/// IBytes(82854982) -> 79 MiB
pub fn ibytes(s: u64) -> String {
    humanate_bytes(s, 1024.0, ["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB"])
}

/// parse_bytes parses a string representation of bytes into the number of bytes it represents
///
/// parse_bytes("42 MB") -> Ok(42000000)
/// parse_bytes("42 mib") -> Ok(44040192)
pub fn parse_bytes(s: &str) -> Result<u64, ParseBytesError> {
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
    let mut tn = String::new();
    if has_comma {
        tn = num.replace(",", "");
    }

    let f = tn.parse::<f64>()
        .context(ParseFloatFailed)?;
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
        _ => return Err(ParseBytesError::UnknownUnit { unit: extra.clone() })
    };

    let f = (f * m as f64) as u64;
    if f > u64::MAX {
        return Err(ParseBytesError::TooLarge { input: s.to_string() });
    }

    Ok(f as u64)
}

#[inline]
fn logn(n: f64, b: f64) -> f64 {
    n.log2() / b.log2()
}

fn humanate_bytes(s: u64, base: f64, sizes: [&str; 7]) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bytes() {
        let input = 82854982u64;

        let s = bytes(input);
        println!("{}", s);
    }

    #[test]
    fn test_ibytes() {
        let input = 82854982u64;

        let s = ibytes(input);
        println!("{}", s);
    }

    #[test]
    fn test_parse_bytes() {
        let tests = [
            ("42MB", 42000000)
        ];

        for (input, want) in tests {
            let value = parse_bytes(input).unwrap();
            println!("{}", value);
        }
    }
}