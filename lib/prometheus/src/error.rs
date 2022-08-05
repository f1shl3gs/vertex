use std::num::{ParseFloatError, ParseIntError};

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub enum Error {
    InvalidHelp {},
    InvalidType(String),
    InvalidMetric(String),

    MissingValue,
    MissingQuantile(String),
    InvalidQuantile { line: String, err: ParseFloatError },
    MissingBucket(String),
    InvalidBucket { line: String, err: ParseFloatError },

    InvalidValue { err: ParseFloatError },
    InvalidTimestamp { err: ParseIntError },

    MultipleMetricKinds { name: String },
    RequestNoNameLabel,
    ValueOutOfRange { value: f64 },
}

impl From<ParseFloatError> for Error {
    fn from(err: ParseFloatError) -> Self {
        Self::InvalidValue { err }
    }
}

impl From<ParseIntError> for Error {
    fn from(err: ParseIntError) -> Self {
        Self::InvalidTimestamp { err }
    }
}
