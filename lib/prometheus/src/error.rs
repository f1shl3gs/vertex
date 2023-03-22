use std::fmt::{Display, Formatter};
use std::num::{ParseFloatError, ParseIntError};

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub enum Error {
    InvalidType(String),
    InvalidMetric(String),

    MissingValue,
    MissingQuantile(String),
    InvalidQuantile { line: String, err: ParseFloatError },
    MissingBucket(String),
    InvalidBucket { line: String, err: ParseFloatError },

    InvalidMetricValue(ParseFloatError),
    InvalidTimestamp(ParseIntError),

    MultipleMetricKinds { name: String },
    RequestNoNameLabel,
    ValueOutOfRange(f64),
}

impl From<ParseFloatError> for Error {
    fn from(err: ParseFloatError) -> Self {
        Self::InvalidMetricValue(err)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidType(typ) => write!(f, "invalid metric type {typ}"),
            Error::InvalidMetric(name) => write!(f, "invalid metric name {name}"),
            Error::MissingValue => f.write_str("value is not found"),
            Error::MissingQuantile(line) => write!(f, "quantile label is not found in `{line}`"),
            Error::InvalidQuantile { line, err } => {
                write!(f, "invalid quantile label value {}, err: {}", line, err)
            }
            Error::MissingBucket(line) => write!(f, "bucket label is not found in `{line}`"),
            Error::InvalidBucket { line, err } => {
                write!(f, "invalid bucket value in {line}, err: {err}")
            }
            Error::InvalidMetricValue(err) => write!(f, "invalid metric value, {err}"),
            Error::InvalidTimestamp(err) => write!(f, "invalid timestamp {err}"),
            Error::MultipleMetricKinds { name } => write!(f, "metric `{name}` have multiple kind"),
            Error::RequestNoNameLabel => f.write_str("metric name label not found"),
            Error::ValueOutOfRange(value) => write!(f, "metric value `{value}` is out of range"),
        }
    }
}
