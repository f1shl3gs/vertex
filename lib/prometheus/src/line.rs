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

#[derive(Debug, Eq, PartialEq)]
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

