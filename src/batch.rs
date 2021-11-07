use std::marker::PhantomData;
use serde::{Deserialize, Serialize};
use snafu::Snafu;

use crate::config::{deserialize_duration, serialize_duration};


#[derive(Debug, Snafu, PartialEq)]
pub enum BatchError {
    #[snafu(display("This sink does not allow setting `max_bytes`"))]
    BytesNotAllowed,
    #[snafu(display("`max_bytes` was unexpectedly zero"))]
    InvalidMaxBytes,
    #[snafu(display("`max_events` was unexpectedly zero"))]
    InvalidMaxEvents,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BatchConfig {
    pub max_bytes: Option<usize>,
    pub max_events: Option<usize>,
    #[serde(deserialize_with = "deserialize_duration", serialize_with = "serialize_duration")]
    pub timeout: Option<chrono::Duration>,
}

impl BatchConfig {
    pub const fn disallow_max_bytes(&self) -> Result<Self, BatchError> {
        // Sinks that used `max_size` for an event count cannot count bytes,
        // so err if `max_bytes` is set
        match self.max_bytes {
            Some(_) => Err(BatchError::BytesNotAllowed),
            None => Ok(*self)
        }
    }

    pub const fn limit_max_bytes(self, limit: usize) -> Self {
        if let Some(n) = self.max_bytes {
            if n > limit {
                return Self {
                    max_bytes: Some(limit),
                    ..self
                }
            }
        }

        self
    }
}

pub struct BatchSize<B> {
    pub bytes: usize,
    pub events: usize,
    _phantom: PhantomData<B>,
}

