use std::fmt::{Display, Formatter};
use std::time::Duration;

const NANOSECOND: u64 = 1;
const MICROSECOND: u64 = 1000 * NANOSECOND;
const MILLISECOND: u64 = 1000 * MICROSECOND;
const SECOND: u64 = 1000 * MILLISECOND;
const MINUTE: u64 = 60 * SECOND;
const HOUR: u64 = 60 * MINUTE;
const DAY: u64 = 24 * HOUR;
const WEEK: u64 = 7 * DAY;

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
fn leading_int(s: &[u8]) -> Result<(u64, &[u8]), ParseDurationError> {
    let mut consumed = 0;
    let o = s
        .iter()
        .take_while(|c| **c >= b'0' && **c <= b'9')
        .try_fold(0u64, |x, &c| {
            consumed += 1;

            if x > u64::MAX / 10 {
                None
            } else {
                Some(10 * x + c as u64 - b'0' as u64)
            }
        });

    match o {
        Some(v) => Ok((v, &s[consumed..])),
        None => Err(ParseDurationError::BadInteger),
    }
}

/// leading_fraction consumes the leader [0-9]* from s.
/// It is used only for fractions, so does not return an error on overflow,
/// it just stops accumulating precision.
fn leading_fraction(s: &[u8]) -> (i64, f64, &[u8]) {
    let mut consumed = 0;
    let mut scale = 1.0;
    let mut overflow = false;

    let o = s
        .iter()
        .take_while(|c| **c >= b'0' && **c <= b'9')
        .try_fold(0, |x, &c| {
            consumed += 1;

            if overflow {
                return Some(x);
            }

            if x > i64::MAX / 10 {
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
        })
        .unwrap();

    (o, scale, &s[consumed..])
}

/// parse_duration parses a duration string.
/// A duration string is a possibly signed sequence of decimal numbers,
/// each with optional fraction and a unit suffix, such as "300ms", "-1.5h" or "2h45m".
/// Valid time units are "ns", "us" (or "µs"), "ms", "s", "m", "h".
pub fn parse_duration(text: &str) -> Result<Duration, ParseDurationError> {
    let mut d = 0u64;
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

    if neg {
        return Err(ParseDurationError::InvalidDuration);
    }

    // Special case: if all that is left is "0", this is zero
    if s.len() == 1 && s[0] == b'0' {
        return Ok(Duration::from_secs(0));
    }

    if s.is_empty() {
        return Err(ParseDurationError::InvalidDuration);
    }

    while !s.is_empty() {
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
        let mut v = l;
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

        if v > u64::MAX / unit {
            return Err(ParseDurationError::InvalidDuration);
        }

        v *= unit;
        if f > 0 {
            // float64 is needed to be nanosecond accurate for fractions of hours.
            // v >= 0 && (f * unit / scale) <= 3.6e+12 (ns/h, h is the largest unit)
            v += (f as f64 * (unit as f64 / scale)) as u64;
            if v < 0 {
                return Err(ParseDurationError::InvalidDuration);
            }
        }

        d += v;
        if d < 0 {
            return Err(ParseDurationError::InvalidDuration);
        }
    }

    Ok(Duration::from_nanos(d))
}

/// to_string returns a string representing the duration in the form "72h3m0.5s".
/// Leading zero units are omitted. As a special case, durations less than one
/// second format use a smaller unit (milli-, micro-, or nanoseconds) to ensure
/// that the leading digit is non-zero. The zero duration formats as 0s
pub fn duration_to_string(d: &Duration) -> String {
    // Largest time is 2540400h10m10.000000000s
    let mut w = 32;
    let mut buf = [0u8; 32];

    let d = d.as_nanos() as u64;
    let mut u = d as u64;
    let neg = d < 0;
    /*    if neg {
        u = -u;
    }*/

    if u < SECOND as u64 {
        // Special case: if duration is smaller thant a second,
        // use smaller units, like 1.2ms
        let mut prec = 0;
        w -= 1;
        buf[w] = b's';
        w -= 1;

        if u == 0 {
            return "0s".to_string();
        } else if u < MICROSECOND as u64 {
            // print nanoseconds
            prec = 0;
            buf[w] = b'n';
        } else if u < MILLISECOND as u64 {
            // print microseconds
            prec = 3;
            /*
            // U+00B5 'µ' micro sign == 0xC2 0xB5
            w -= 1; // Need room for two bytes
            buf[w + 1] = 0xC2;
            buf[w + 2] = 0xB5;
            */
            buf[w] = b'u';
        } else {
            // print milliseconds
            prec = 6;
            buf[w] = b'm';
        }

        let (_w, _u) = fmt_frac(&mut buf[..w], u, prec);
        w = _w;
        u = _u;
        w = fmt_int(&mut buf[..w], u);
    } else {
        w -= 1;
        buf[w] = b's';

        let (_w, _u) = fmt_frac(&mut buf[..w], u, 9);
        w = _w;
        u = _u;

        // u is now integer seconds
        w = fmt_int(&mut buf[..w], u % 60);
        u /= 60;

        // u is now integer minutes
        if u > 0 {
            w -= 1;
            buf[w] = b'm';
            w = fmt_int(&mut buf[..w], u % 60);
            u /= 60;

            // u is now integer hours
            // Stop at hours because days can be different lengths.
            if u > 0 {
                w -= 1;
                buf[w] = b'h';
                w = fmt_int(&mut buf[..w], u)
            }
        }
    }

    if neg {
        w -= 1;
        buf[w] = b'-';
    }

    return String::from_utf8_lossy(&buf[w..]).to_string();
}

// fmt_frac formats the fraction of v / 10 ** prec (e.g., ".12345") into the
// tail of buf, omitting trailing zeros. It omits the decimal point too when
// the fraction is 0. It returns the index where the output bytes begin and
// the value v / 10 ** prec
fn fmt_frac(buf: &mut [u8], mut v: u64, prec: i32) -> (usize, u64) {
    // Omit trailing zeros up to and including decimal point
    let mut w = buf.len();
    let mut print = false;
    for i in 0..prec {
        let digit = v % 10;
        print = print || digit != 0;
        if print {
            w -= 1;
            buf[w] = digit as u8 + b'0';
        }

        v /= 10;
    }

    if print {
        w -= 1;
        buf[w] = b'.';
    }

    (w, v)
}

// fmt_int formats v into the tail of buf.
// It returns the index where the output begins.
fn fmt_int(buf: &mut [u8], mut v: u64) -> usize {
    let mut w = buf.len();
    if v == 0 {
        w -= 1;
        buf[w] = b'0';
    } else {
        while v > 0 {
            w -= 1;
            buf[w] = (v % 10) as u8 + b'0';
            v /= 10;
        }
    }

    return w;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

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
        want: u64,
    }

    #[test]
    fn test_parse_duration() {
        let tests = [
            // simple
            ParseDurationTest {
                input: "0",
                want: 0,
            },
            ParseDurationTest {
                input: "5s",
                want: 5 * SECOND,
            },
            ParseDurationTest {
                input: "30s",
                want: 30 * SECOND,
            },
            ParseDurationTest {
                input: "1478s",
                want: 1478 * SECOND,
            },
            // sign
            // ParseDurationTest { input: "-5s", want: -5 * SECOND },
            ParseDurationTest {
                input: "+5s",
                want: 5 * SECOND,
            },
            // ParseDurationTest { input: "-0", want: 0 },
            ParseDurationTest {
                input: "+0",
                want: 0,
            },
            // decimal
            ParseDurationTest {
                input: "5.0s",
                want: 5 * SECOND,
            },
            ParseDurationTest {
                input: "5.6s",
                want: 5 * SECOND + 600 * MILLISECOND,
            },
            ParseDurationTest {
                input: "5.s",
                want: 5 * SECOND,
            },
            ParseDurationTest {
                input: ".5s",
                want: 500 * MILLISECOND,
            },
            ParseDurationTest {
                input: "1.0s",
                want: 1 * SECOND,
            },
            ParseDurationTest {
                input: "1.00s",
                want: 1 * SECOND,
            },
            ParseDurationTest {
                input: "1.004s",
                want: 1 * SECOND + 4 * MILLISECOND,
            },
            ParseDurationTest {
                input: "1.0040s",
                want: 1 * SECOND + 4 * MILLISECOND,
            },
            ParseDurationTest {
                input: "100.00100s",
                want: 100 * SECOND + 1 * MILLISECOND,
            },
            // different units
            ParseDurationTest {
                input: "10ns",
                want: 10 * NANOSECOND,
            },
            ParseDurationTest {
                input: "11us",
                want: 11 * MICROSECOND,
            },
            ParseDurationTest {
                input: "12µs",
                want: 12 * MICROSECOND,
            }, // U+00B5
            ParseDurationTest {
                input: "12µs10ns",
                want: 12 * MICROSECOND + 10 * NANOSECOND,
            }, // U+00B5
            ParseDurationTest {
                input: "12μs",
                want: 12 * MICROSECOND,
            }, // U+03BC
            ParseDurationTest {
                input: "12μs10ns",
                want: 12 * MICROSECOND + 10 * NANOSECOND,
            }, // U+03BC
            ParseDurationTest {
                input: "13ms",
                want: 13 * MILLISECOND,
            },
            ParseDurationTest {
                input: "14s",
                want: 14 * SECOND,
            },
            ParseDurationTest {
                input: "15m",
                want: 15 * MINUTE,
            },
            ParseDurationTest {
                input: "16h",
                want: 16 * HOUR,
            },
            // composite durations
            ParseDurationTest {
                input: "3h30m",
                want: 3 * HOUR + 30 * MINUTE,
            },
            ParseDurationTest {
                input: "10.5s4m",
                want: 4 * MINUTE + 10 * SECOND + 500 * MILLISECOND,
            },
            // ParseDurationTest { input: "-2m3.4s", want: -(2 * MINUTE + 3 * SECOND + 400 * MILLISECOND) },
            ParseDurationTest {
                input: "1h2m3s4ms5us6ns",
                want: 1 * HOUR
                    + 2 * MINUTE
                    + 3 * SECOND
                    + 4 * MILLISECOND
                    + 5 * MICROSECOND
                    + 6 * NANOSECOND,
            },
            ParseDurationTest {
                input: "39h9m14.425s",
                want: 39 * HOUR + 9 * MINUTE + 14 * SECOND + 425 * MILLISECOND,
            },
            // large value
            ParseDurationTest {
                input: "52763797000ns",
                want: 52763797000 * NANOSECOND,
            },
            // more than 9 digits after decimal point, see https://golang.org/issue/6617
            ParseDurationTest {
                input: "0.3333333333333333333h",
                want: 20 * MINUTE,
            },
            // 9007199254740993 = 1<<53+1 cannot be stored precisely in a float64
            // ParseDurationTest {
            //     input: "9007199254740993ns",
            //     want: (1 << 53 + 1) * NANOSECOND,
            // },
            // largest duration that can be represented by int64 in nanoseconds
            // ParseDurationTest { input: "9223372036854775807ns", want: i64::MAX * NANOSECOND },
            // ParseDurationTest { input: "9223372036854775.807us", want: i64::MAX * NANOSECOND },
            // ParseDurationTest { input: "9223372036s854ms775us807ns", want: i64::MAX * NANOSECOND },
            // large negative value
            // todo: ParseDurationTest { input: "-9223372036854775807ns", want: -1 << 63 + 1 * NANOSECOND },
            // huge string; issue 15011.
            ParseDurationTest {
                input: "0.100000000000000000000h",
                want: 6 * MINUTE,
            },
            // This value tests the first overflow check in leadingFraction.
            ParseDurationTest {
                input: "0.830103483285477580700h",
                want: 49 * MINUTE + 48 * SECOND + 372539827 * NANOSECOND,
            },
        ];

        for test in tests {
            let d = parse_duration(&test.input).unwrap();
            assert_eq!(d, Duration::from_nanos(test.want), "input: {}", test.input);
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
    fn test_leading_fraction() {
        let (f, scale, r) = leading_fraction("6s".as_bytes());
        assert_eq!(6, f);
        assert_eq!(10.0, scale);
        assert_eq!(r, "s".as_bytes());
    }

    #[test]
    fn test_duration_to_string() {
        let tests = vec![
            ("0s", 0),
            ("1ns", 1 * NANOSECOND),
            // ("1.1µs", 1100 * NANOSECOND),
            ("1.1us", 1100 * NANOSECOND),
            ("2.2ms", 2200 * MICROSECOND),
            ("3.3s", 3300 * MILLISECOND),
            ("4m5s", 4 * MINUTE + 5 * SECOND),
            ("4m5.001s", 4 * MINUTE + 5001 * MILLISECOND),
            ("5h6m7.001s", 5 * HOUR + 6 * MINUTE + 7001 * MILLISECOND),
            ("8m0.000000001s", 8 * MINUTE + 1 * NANOSECOND),
            // ("2562047h47m16.854775807s", u64::MAX),
            // ("-2562047h47m16.854775808s", u64::MIN),
        ];

        for (want, input) in tests {
            let duration = Duration::from_nanos(input);
            assert_eq!(duration_to_string(&duration), want)
        }
    }
}
