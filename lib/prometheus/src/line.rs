use std::collections::BTreeMap;
/// Parse a single line of Prometheus text format

use nom::{branch::alt, bytes::complete::{is_not, tag, take_while, take_while1}, character::complete::{char, digit1}, combinator::{map, opt, recognize, value}, Err, error::ParseError, multi::fold_many0, number::complete::double, sequence::{delimited, pair, preceded, tuple}};

#[derive(Debug, snafu::Snafu, PartialEq)]
pub enum ErrorKind {
    #[snafu(display("invalid metric type, parsing: {}", input))]
    InvalidMetricKind { input: String },
    #[snafu(display("excepted token {:?}, parsing: {}", expected, input))]
    ExceptedToken {
        expected: &'static str,
        input: String,
    },
    #[snafu(display("expected blank space or tab, parsing {}", input))]
    ExpectedSpace { input: String },
    #[snafu(display("expected char {:?}, parsing: {}", expected, input))]
    ExpectedChar { expected: char, input: String },
    #[snafu(display("name must start with [a-zA-Z_], parsing: {}", input))]
    ParseNameError { input: String },
    #[snafu(display("parse float value error, parsing: {}", input))]
    ParseFloatError { input: String },
    #[snafu(display("parse timestamp error, parsing: {}", input))]
    ParseTimestampError { input: String },

    // Error that we didn't catch
    #[snafu(display("error kind: {:?}, parsing: {}", kind, input))]
    Nom { input: String, kind: nom::error::ErrorKind },
}

/// We try to catch all nom's `ErrorKind` with our own `ErrorKind`,
/// to provide a meaningful error message.
/// Parsers in this module should return this IResult instead of
/// `nom::IResult`
type IResult<'a, O> = Result<(&'a str, O), nom::Err<ErrorKind>>;

impl From<ErrorKind> for nom::Err<ErrorKind> {
    fn from(err: ErrorKind) -> Self {
        nom::Err::Error(err)
    }
}

impl From<nom::Err<ErrorKind>> for ErrorKind {
    fn from(err: Err<ErrorKind>) -> Self {
        match err {
            // this error only occurs when "streaming" nom is used
            nom::Err::Incomplete(_) => unreachable!(),
            nom::Err::Error(e) | nom::Err::Failure(e) => e
        }
    }
}

impl<'a> nom::error::ParseError<&'a str> for ErrorKind {
    fn from_error_kind(input: &'a str, kind: nom::error::ErrorKind) -> Self {
        ErrorKind::Nom {
            input: input.to_owned(),
            kind,
        }
    }

    fn append(_input: &'a str, _kind: nom::error::ErrorKind, other: Self) -> Self {
        other
    }
}

type NomErrorType<'a> = (&'a str, nom::error::ErrorKind);

type NomError<'a> = nom::Err<NomErrorType<'a>>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MetricKind {
    Counter,
    Gauge,
    Histogram,
    Summary,
    Untyped,
}

#[derive(Debug, PartialEq)]
pub struct Header {
    pub metric_name: String,
    pub kind: MetricKind,
}

impl Header {
    fn space1(input: &str) -> IResult<()> {
        take_while1(|c| c == ' ' || c == '\t')(input)
            .map_err(|_: NomError| {
                ErrorKind::ExpectedSpace {
                    input: input.to_owned()
                }.into()
            })
            .map(|(input, _)| (input, ()))
    }

    /// `# TYPE <metric_name> <metric_type>`
    fn parse(input: &str) -> IResult<Self> {
        let input = trim_space(input);
        let (input, _) = char('#')(input)
            .map_err(|_: NomError| ErrorKind::ExpectedChar {
                expected: '#',
                input: input.to_owned(),
            })?;
        let input = trim_space(input);
        let (input, _) = tag("TYPE")(input)
            .map_err(|_: NomError| ErrorKind::ExceptedToken {
                expected: "TYPE",
                input: input.to_owned(),
            })?;
        let (input, _) = Self::space1(input)?;
        let (input, metric_name) = parse_name(input)?;
        let (input, _) = Self::space1(input)?;
        let (input, kind) = alt((
            value(MetricKind::Counter, tag("counter")),
            value(MetricKind::Gauge, tag("gauge")),
            value(MetricKind::Summary, tag("summary")),
            value(MetricKind::Histogram, tag("histogram")),
            value(MetricKind::Untyped, tag("untyped")),
        ))(input)
            .map_err(|_: NomError| ErrorKind::InvalidMetricKind {
                input: input.to_owned()
            })?;
        Ok((input, Header { metric_name, kind }))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Metric {
    pub name: String,
    pub labels: BTreeMap<String, String>,
    pub value: f64,
    pub timestamp: Option<i64>,
}

impl Metric {
    /// Parse a single line with format
    ///
    /// ```text
    /// metric_name [
    ///   "{" label_name "=" `"` label_value `"` { "," label_name "=" `"` label_value `"` } [ "," ] "}"
    /// ] value [ timestamp ]
    /// ```
    ///
    /// We don't parse timestamp
    fn parse(input: &str) -> IResult<Self> {
        let input = trim_space(input);
        let (input, name) = parse_name(input)?;
        let (input, labels) = Self::parse_labels(input)?;
        let (input, value) = Self::parse_value(input)?;
        let (input, timestamp) = Self::parse_timestamp(input)?;

        Ok((input, Metric {
            name,
            labels,
            value,
            timestamp,
        }))
    }

    /// Float value, and +Inf, -Int, Nan
    pub fn parse_value(input: &str) -> IResult<f64> {
        let input = trim_space(input);
        alt((
            value(f64::INFINITY, tag("+Inf")),
            value(f64::NEG_INFINITY, tag("-Inf")),
            value(f64::NAN, tag("Nan")),
            // Note see https://github.com/Geal/nom/issues/1384
            // This shouldn't be necessary if that issue is remedied
            value(f64::NAN, tag("NaN")),
            double,
        ))(input)
            .map_err(|_: NomError| {
                ErrorKind::ParseFloatError {
                    input: input.to_owned(),
                }.into()
            })
    }

    fn parse_name_value(input: &str) -> IResult<(String, String)> {
        map(
            tuple((parse_name, match_char('='), Self::parse_escaped_string)),
            |(name, _, value)| (name, value),
        )(input)
    }

    // Return:
    // - Some((name, value)) => success
    // - None => list is properly ended with "}"
    // - Error => errors of parse_name_value
    fn element_parser(input: &str) -> IResult<Option<(String, String)>> {
        match Self::parse_name_value(input) {
            Ok((input, result)) => Ok((input, Some(result))),
            Err(nom::Err::Error(parse_name_value_error)) => {
                match match_char('}')(input) {
                    Ok((input, _)) => Ok((input, None)),
                    Err(nom::Err::Error(_)) => Err(nom::Err::Error(parse_name_value_error)),
                    Err(err) => Err(err)
                }
            }
            Err(err) => Err(err)
        }
    }

    fn parse_labels_inner(mut input: &str) -> IResult<BTreeMap<String, String>> {
        let sep = match_char(',');
        let mut result = BTreeMap::new();

        loop {
            match Self::element_parser(input)? {
                (inner_input, None) => {
                    input = inner_input;
                    break;
                }

                (inner_input, Some((name, value))) => {
                    result.insert(name, value);

                    // try matching ",", if doesn't match then check if
                    // the list ended with "}". If not ended then return
                    // error `expected token ','`.
                    let inner_input = match sep(inner_input) {
                        Ok((inner_input, _)) => inner_input,
                        Err(sep_err) => match match_char('}')(inner_input) {
                            Ok((inner_input, _)) => {
                                input = inner_input;
                                break;
                            }
                            Err(_) => return Err(sep_err)
                        }
                    };

                    input = inner_input;
                }
            }
        }

        Ok((input, result))
    }

    /// Parse `{label_name="value",...}`
    fn parse_labels(input: &str) -> IResult<BTreeMap<String, String>> {
        let input = trim_space(input);

        match opt(char('{'))(input) {
            Ok((input, None)) => Ok((input, BTreeMap::new())),
            Ok((input, Some(_))) => Self::parse_labels_inner(input),
            Err(err) => Err(err)
        }
    }

    fn parse_timestamp(input: &str) -> IResult<Option<i64>> {
        let input = trim_space(input);
        opt(map(recognize(pair(opt(char('-')), digit1)), |s: &str| {
            s.parse().unwrap()
        }))(input)
    }

    /// Parse `'"' string_content '"'`. `string_content` can contain any unicode characters,
    /// backslash (`\`), double-quote (`"`), and line feed (`\n`) characters have to be
    /// escaped as `\\`, `\"` and `\n`, respectively
    fn parse_escaped_string(input: &str) -> IResult<String> {
        #[derive(Debug)]
        enum StringFragment<'a> {
            Literal(&'a str),
            EscapedChar(char),
        }

        let parse_string_fragement = alt((
            map(is_not("\"\\"), StringFragment::Literal),
            map(
                preceded(
                    char('\\'),
                    alt((
                        value('\n', char('n')),
                        value('"', char('"')),
                        value('\\', char('\\')),
                    )),
                ),
                StringFragment::EscapedChar,
            ),
        ));

        let input = trim_space(input);
        let build_string = fold_many0(
            parse_string_fragement,
            String::new,
            |mut result, fragment| {
                match fragment {
                    StringFragment::Literal(s) => result.push_str(s),
                    StringFragment::EscapedChar(c) => result.push(c),
                }

                result
            },
        );

        fn match_quote(input: &str) -> IResult<char> {
            char('"')(input).map_err(|_: NomError| {
                ErrorKind::ExpectedChar {
                    expected: '"',
                    input: input.to_owned(),
                }.into()
            })
        }

        delimited(match_quote, build_string, match_quote)(input)
    }
}

fn trim_space(input: &str) -> &str {
    input.trim_start_matches(|c| c == ' ' || c == '\t')
}

fn sp<'a, E: ParseError<&'a str>>(i: &'a str) -> nom::IResult<&'a str, &'a str, E> {
    take_while(|c| c == ' ' || c == '\t')(i)
}

fn match_char(c: char) -> impl Fn(&str) -> IResult<char> {
    move |input| {
        preceded(sp, char(c))(input).map_err(|_: NomError| {
            ErrorKind::ExpectedChar {
                expected: c,
                input: input.to_owned(),
            }.into()
        })
    }
}

/// Name matches the regex `[a-zA-Z_][a-zA-Z0-9_]*`
fn parse_name(input: &str) -> IResult<String> {
    let input = trim_space(input);
    let (input, (a, b)) = pair(
        take_while1(|c: char| c.is_alphabetic() || c == '_'),
        take_while(|c: char| c.is_alphanumeric() || c == '_' || c == ':'),
    )(input)
        .map_err(|_: NomError| ErrorKind::ParseNameError {
            input: input.to_owned()
        })?;

    Ok((input, a.to_owned() + b))
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! btreemap {
        () => (::std::collections::BTreeMap::new());

        // trailing comma case
        ($($key:expr => $value:expr,)+) => (btreemap!($($key => $value),+));

        ($($key:expr => $value:expr),*) => {
            {
                let mut _map = ::std::collections::BTreeMap::new();
                $(
                    let _ = _map.insert($key.into(), $value.into());
                )*
                _map
            }
        };
    }

    #[test]
    fn parse_escaped_string() {
        fn wrap(s: &str) -> String {
            format!("  \t \"{}\"  .", s)
        }

        // parser should not consume more that it needed
        let tail = "  .";

        let input = wrap("");
        let (left, r) = Metric::parse_escaped_string(&input).unwrap();
        assert_eq!(left, tail);
        assert_eq!(r, "");

        let input = wrap(r#"a\\ asdf"#);
        let (left, r) = Metric::parse_escaped_string(&input).unwrap();
        assert_eq!(left, tail);
        assert_eq!(r, "a\\ asdf");

        let input = wrap(r#"\"\""#);
        let (left, r) = Metric::parse_escaped_string(&input).unwrap();
        assert_eq!(left, tail);
        assert_eq!(r, "\"\"");

        let input = wrap(r#"\"\\\n"#);
        let (left, r) = Metric::parse_escaped_string(&input).unwrap();
        assert_eq!(left, tail);
        assert_eq!(r, "\"\\\n");

        let input = wrap(r#"\\n"#);
        let (left, r) = Metric::parse_escaped_string(&input).unwrap();
        assert_eq!(left, tail);
        assert_eq!(r, "\\n");

        let input = wrap(r#"  😂  "#);
        let (left, r) = Metric::parse_escaped_string(&input).unwrap();
        assert_eq!(left, tail);
        assert_eq!(r, "  😂  ");
    }

    #[test]
    fn test_parse_name() {
        let tail = "  .";

        fn wrap(s: &str) -> String {
            format!("  \t {}  .", s)
        }

        struct Case {
            input: String,
            want: String,
            err: bool,
        }

        for case in vec![
            Case {
                input: wrap("abc_def"),
                want: "abc_def".into(),
                err: false,
            },
            Case {
                input: wrap("__9A0bc_def__"),
                want: "__9A0bc_def__".into(),
                err: false,
            },
            Case {
                input: wrap("consul_serf_events_consul:new_leader"),
                want: "consul_serf_events_consul:new_leader".into(),
                err: false,
            },
            Case {
                input: wrap("99"),
                want: "".into(),
                err: true,
            },
        ] {
            let result = parse_name(&case.input);
            match case.err {
                true => assert!(result.is_err()),
                _ => {
                    let (left, r) = result.unwrap();
                    assert_eq!(left, tail);
                    assert_eq!(case.want, r);
                }
            }
        }
    }

    #[test]
    fn test_parse_header() {
        struct Case {
            input: String,
            want: Header,
        }

        fn wrap(s: &str) -> String {
            format!("  \t {}  .", s)
        }
        let tail = "  .";

        for case in vec![
            Case {
                input: wrap("#  TYPE abc_def counter"),
                want: Header {
                    metric_name: "abc_def".into(),
                    kind: MetricKind::Counter,
                },
            },
            Case {
                input: wrap("#  TYPE abc_def counteraaaaaaaaaaa"),
                want: Header {
                    metric_name: "abc_def".into(),
                    kind: MetricKind::Counter,
                },
            },
            Case {
                input: wrap("#TYPE \t abc_def \t gauge"),
                want: Header {
                    metric_name: "abc_def".into(),
                    kind: MetricKind::Gauge,
                },
            },
            Case {
                input: wrap("# TYPE abc_def histogram"),
                want: Header {
                    metric_name: "abc_def".into(),
                    kind: MetricKind::Histogram,
                },
            },
            Case {
                input: wrap("# TYPE abc_def summary"),
                want: Header {
                    metric_name: "abc_def".into(),
                    kind: MetricKind::Summary,
                },
            },
            Case {
                input: wrap("# TYPE abc_def untyped"),
                want: Header {
                    metric_name: "abc_def".into(),
                    kind: MetricKind::Untyped,
                },
            },
        ] {
            let (left, r) = Header::parse(&case.input).unwrap();
            assert_eq!(left, tail);
            assert_eq!(r, case.want);
        }
    }

    #[test]
    fn test_parse_value() {
        let tail = "  .";
        fn wrap(s: &str) -> String {
            format!("  \t {}  .", s)
        }

        let cases = [
            ("0", 0.0f64),
            ("0.25", 0.25f64),
            ("-10.25", -10.25f64),
            ("-10e-25", -10e-25f64),
            ("-10e+25", -10e+25f64),
            ("2020", 2020.0f64),
            ("1.", 1f64),
        ];
        for (input, want) in cases {
            let input = wrap(input);
            let (left, r) = Metric::parse_value(&input).unwrap();
            assert_eq!(left, tail);
            assert_eq!(r, want)
        }
    }

    #[test]
    fn test_infinite_and_nan() {
        fn wrap(s: &str) -> String {
            format!("  \t {}  .", s)
        }
        let tail = "  .";

        let input = wrap("+Inf");
        let (left, r) = Metric::parse_value(&input).unwrap();
        assert_eq!(left, tail);
        assert!(r.is_infinite() && r.is_sign_positive());

        let input = wrap("-Inf");
        let (left, r) = Metric::parse_value(&input).unwrap();
        assert_eq!(left, tail);
        assert!(r.is_infinite() && r.is_sign_negative());

        let input = wrap("Nan");
        let (left, r) = Metric::parse_value(&input).unwrap();
        assert_eq!(left, tail);
        assert!(r.is_nan());
    }

    #[test]
    fn test_parse_labels() {
        let tail = "  .";
        fn wrap(s: &str) -> String {
            format!("  \t {}  .", s)
        }

        let tests = [
            ("{}", btreemap!()),
            (r#"{name="value"}"#, btreemap!( "name" => "value")),
            (r#"{name="value",}"#, btreemap!( "name" => "value")),
            (r#"{name = "value" , key ="value"}"#, btreemap!(
                "name" => "value",
                "key" => "value"
            )),
            (r#"{ name = "" ,b="a=b" , a="},", _c = "\""}"#, btreemap!(
                "name" => "",
                "a" => "},",
                "b" => "a=b",
                "_c" => "\""
            ))
        ];

        for (input, want) in tests {
            let input = wrap(input);
            let (left, labels) = Metric::parse_labels(&input).unwrap();
            assert_eq!(left, tail);
            assert_eq!(labels, want)
        }

        let input = wrap("100");
        let (left, labels) = Metric::parse_labels(&input).unwrap();
        assert_eq!(left, "100".to_owned() + tail);
        assert_eq!(labels, btreemap!());

        // We don't allow theos values
        let input = wrap(r#"{name="value}"#);
        let err = Metric::parse_labels(&input).unwrap_err().into();
        assert!(matches!(
            err,
            ErrorKind::ExpectedChar { expected: '"', .. }
        ));

        let input = wrap(r#"{ a="b" c = "d" }"#);
        let err = Metric::parse_labels(&input).unwrap_err().into();
        assert!(matches!(err, ErrorKind::ExpectedChar { expected: ',', ..}));

        let input = wrap(r#"{ a="b" ,, c="d"}"#);
        let err = Metric::parse_labels(&input).unwrap_err().into();
        assert!(matches!(err, ErrorKind::ParseNameError { .. }))
    }
}