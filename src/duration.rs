use std::fmt::{Display, Formatter};

const NANOSECOND: i64 = 1;
const MICROSECOND: i64 = 1000 * NANOSECOND;
const MILLISECOND: i64 = 1000 * MICROSECOND;
const SECOND: i64 = 1000 * MILLISECOND;
const MINUTE: i64 = 60 * SECOND;
const HOUR: i64 = 60 * MINUTE;
const DAY: i64 = 24 * HOUR;
const WEEK: i64 = 7 * DAY;

#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub enum ParseDurationError {
    BadInteger,
    InvalidDuration,
    MissingUnit,
    UnknownUnit,
}

impl Display for ParseDurationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

/// leading_int consumes the leading [0-9]* from s
fn leading_int(s: &[u8]) -> Result<(i64, &[u8]), ParseDurationError> {
    let mut consumed = 0;
    let o = s.iter()
        .take_while(|c| **c >= b'0' && **c <= b'9')
        .try_fold(0i64, |x, &c| {
            consumed += 1;

            if x > (1 << 63 - 1) / 10 {
                None
            } else {
                Some(10 * x + c as i64 - b'0' as i64)
            }
        });

    match o {
        Some(v) => Ok((v, &s[consumed..])),
        None => Err(ParseDurationError::BadInteger)
    }
}

/// leading_fraction consumes the leader [0-9]* from s.
/// It is used only for fractions, so does not return an error on overflow,
/// it just stops accumulating precision.
fn leading_fraction(s: &[u8]) -> (i64, f64, &[u8]) {
    let mut consumed = 0;
    let mut scale = 1.0;
    let mut overflow = false;

    let o = s.iter()
        .take_while(|c| **c >= b'0' && **c <= b'9')
        .try_fold(0, |x, &c| {
            consumed += 1;

            if overflow {
                return Some(x);
            }

            if x > (1 << 63 - 1) / 10 {
                overflow = true;
                return Some(x);
            }

            let y = x * 10 + c as i64 - b'0' as i64;
            if y < 0 {
                overflow = true;
                return Some(x);
            }

            scale *= 10.0;
            Some(y)
        }).unwrap();

    (o, scale, &s[consumed..])
}

/// parse_duration parses a duration string.
/// A duration string is a possibly signed sequence of decimal numbers,
/// each with optional fraction and a unit suffix, such as "300ms", "-1.5h" or "2h45m".
/// Valid time units are "ns", "us" (or "µs"), "ms", "s", "m", "h".
pub fn parse_duration(text: &str) -> Result<chrono::Duration, ParseDurationError> {
    let mut d = 0;
    let mut neg = false;
    let mut s = text.as_bytes();

    // Consume [-+]?
    if !s.is_empty() {
        let c = s[0];
        if c == b'-' || c == b'+' {
            neg = c == b'-';
            s = &s[1..];
        }
    }

    // Special case: if all that is left is "0", this is zero
    if s.len() == 1 && s[0] == b'0' {
        return Ok(chrono::Duration::seconds(0));
    }

    if s.is_empty() {
        return Err(ParseDurationError::InvalidDuration);
    }

    while !s.is_empty() {
        let mut v = 0;
        let mut f = 0;
        let mut scale = 1.0;

        // The next character must be [0-9.]
        let c = s[0];
        if !(c == b'.' || b'0' <= c && c <= b'9') {
            return Err(ParseDurationError::InvalidDuration);
        }

        // Consume [0-9]*
        let pl = s.len();
        let (l, remain) = leading_int(s)?;
        v = l;
        s = remain;
        let pre = pl != s.len();

        // Consume (\.[0-9]*)?
        let mut post = false;
        if !s.is_empty() && s[0] == b'.' {
            s = &s[1..];
            let pl = s.len();
            let (lf, ls, remain) = leading_fraction(s);
            f = lf;
            scale = ls;
            s = remain;
            post = pl != s.len();
        }
        if !pre && !post {
            // no digits (e.g. ".s" or "-.s")
            return Err(ParseDurationError::InvalidDuration);
        }

        // Consume unit
        let mut i = 0;
        while i < s.len() {
            let c = s[i];
            if c == b'.' || (b'0'..=b'9').contains(&c) {
                break;
            }

            i += 1;
        }

        if i == 0 {
            return Err(ParseDurationError::MissingUnit);
        }
        let u = &s[..i];
        s = &s[i..];
        let unit = match u {
            [b'n', b's'] => NANOSECOND,
            [b'u', b's'] => MICROSECOND,
            // "µs" U+00B5
            [194, 181, 115] => MICROSECOND,
            // "μs" U+03BC
            [206, 188, 115] => MICROSECOND,
            [b'm', b's'] => MILLISECOND,
            [b's'] => SECOND,
            [b'm'] => MINUTE,
            [b'h'] => HOUR,
            [b'd'] => DAY,
            [b'w'] => WEEK,
            _ => 0,
        };
        if unit == 0 {
            return Err(ParseDurationError::UnknownUnit);
        }

        if v > (1 << 63 - 1) / unit {
            return Err(ParseDurationError::InvalidDuration);
        }

        v *= unit;
        if f > 0 {
            // float64 is needed to be nanosecond accurate for fractions of hours.
            // v >= 0 && (f * unit / scale) <= 3.6e+12 (ns/h, h is the largest unit)
            v += (f as f64 * (unit as f64 / scale)) as i64;
            if v < 0 {
                return Err(ParseDurationError::InvalidDuration);
            }
        }

        d += v;
        if d < 0 {
            return Err(ParseDurationError::InvalidDuration);
        }
    }

    if neg {
        d = -d
    }

    Ok(chrono::Duration::nanoseconds(d))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_leading_int() {
        let (x, remain) = leading_int("12h".as_bytes()).unwrap();
        println!("{} {}", x, String::from_utf8_lossy(remain));
    }

    #[test]
    fn test_leading_int_overflow() {
        let err = leading_int("999999999999999999999".as_bytes()).unwrap_err();
        assert_eq!(err, ParseDurationError::BadInteger)
    }

    struct ParseDurationTest {
        input: &'static str,
        want: i64,
    }

    #[test]
    fn test_parse_duration() {
        let tests = [
            // simple
            ParseDurationTest { input: "0", want: 0 },
            ParseDurationTest { input: "5s", want: 5 * SECOND },
            ParseDurationTest { input: "30s", want: 30 * SECOND },
            ParseDurationTest { input: "1478s", want: 1478 * SECOND },
            // sign
            ParseDurationTest { input: "-5s", want: -5 * SECOND },
            ParseDurationTest { input: "+5s", want: 5 * SECOND },
            ParseDurationTest { input: "-0", want: 0 },
            ParseDurationTest { input: "+0", want: 0 },
            // decimal
            ParseDurationTest { input: "5.0s", want: 5 * SECOND },
            ParseDurationTest { input: "5.6s", want: 5 * SECOND + 600 * MILLISECOND },
            ParseDurationTest { input: "5.s", want: 5 * SECOND },
            ParseDurationTest { input: ".5s", want: 500 * MILLISECOND },
            ParseDurationTest { input: "1.0s", want: 1 * SECOND },
            ParseDurationTest { input: "1.00s", want: 1 * SECOND },
            ParseDurationTest { input: "1.004s", want: 1 * SECOND + 4 * MILLISECOND },
            ParseDurationTest { input: "1.0040s", want: 1 * SECOND + 4 * MILLISECOND },
            ParseDurationTest { input: "100.00100s", want: 100 * SECOND + 1 * MILLISECOND },
            // different units
            ParseDurationTest { input: "10ns", want: 10 * NANOSECOND },
            ParseDurationTest { input: "11us", want: 11 * MICROSECOND },
            ParseDurationTest { input: "12µs", want: 12 * MICROSECOND }, // U+00B5
            ParseDurationTest { input: "12µs10ns", want: 12 * MICROSECOND + 10 * NANOSECOND }, // U+00B5
            ParseDurationTest { input: "12μs", want: 12 * MICROSECOND }, // U+03BC
            ParseDurationTest { input: "12μs10ns", want: 12 * MICROSECOND + 10 * NANOSECOND }, // U+03BC
            ParseDurationTest { input: "13ms", want: 13 * MILLISECOND },
            ParseDurationTest { input: "14s", want: 14 * SECOND },
            ParseDurationTest { input: "15m", want: 15 * MINUTE },
            ParseDurationTest { input: "16h", want: 16 * HOUR },
            // composite durations
            ParseDurationTest { input: "3h30m", want: 3 * HOUR + 30 * MINUTE },
            ParseDurationTest { input: "10.5s4m", want: 4 * MINUTE + 10 * SECOND + 500 * MILLISECOND },
            ParseDurationTest { input: "-2m3.4s", want: -(2 * MINUTE + 3 * SECOND + 400 * MILLISECOND) },
            ParseDurationTest { input: "1h2m3s4ms5us6ns", want: 1 * HOUR + 2 * MINUTE + 3 * SECOND + 4 * MILLISECOND + 5 * MICROSECOND + 6 * NANOSECOND },
            ParseDurationTest { input: "39h9m14.425s", want: 39 * HOUR + 9 * MINUTE + 14 * SECOND + 425 * MILLISECOND },
            // large value
            ParseDurationTest { input: "52763797000ns", want: 52763797000 * NANOSECOND },
            // more than 9 digits after decimal point, see https://golang.org/issue/6617
            ParseDurationTest { input: "0.3333333333333333333h", want: 20 * MINUTE },
            // 9007199254740993 = 1<<53+1 cannot be stored precisely in a float64
            ParseDurationTest { input: "9007199254740993ns", want: (1 << 53 + 1) * NANOSECOND },
            // largest duration that can be represented by int64 in nanoseconds
            ParseDurationTest { input: "9223372036854775807ns", want: (1 << 63 - 1) * NANOSECOND },
            ParseDurationTest { input: "9223372036854775.807us", want: (1 << 63 - 1) * NANOSECOND },
            ParseDurationTest { input: "9223372036s854ms775us807ns", want: (1 << 63 - 1) * NANOSECOND },
            // large negative value
            // todo: ParseDurationTest { input: "-9223372036854775807ns", want: -1 << 63 + 1 * NANOSECOND },
            // huge string; issue 15011.
            ParseDurationTest { input: "0.100000000000000000000h", want: 6 * MINUTE },
            // This value tests the first overflow check in leadingFraction.
            ParseDurationTest { input: "0.830103483285477580700h", want: 49 * MINUTE + 48 * SECOND + 372539827 * NANOSECOND }
        ];

        for test in tests {
            println!("input: {}", test.input);
            let d = parse_duration(&test.input).unwrap();
            assert_eq!(d, Duration::nanoseconds(test.want))
        }
    }

    #[test]
    fn parse_us() {
        let input = "12µs"; // U+00B5
        let d = parse_duration(input).unwrap();


        let input = "12μs"; // U+03BC
        let d = parse_duration(input).unwrap();
    }

    #[test]
    fn test_second_with_mill() {
        let d = parse_duration("5.6s").unwrap();
        println!("{}", d)
    }

    #[test]
    fn test_leading_fraction() {
        let (f, scale, r) = leading_fraction("6s".as_bytes());
        assert_eq!(6, f);
        assert_eq!(10.0, scale);
        assert_eq!(r, "s".as_bytes());
    }
}