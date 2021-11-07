mod codec;

use std::io;
use std::io::Write;
use std::sync::Arc;

use crate::log::path_iter::{PathComponent, PathIter};
use crate::{Event, LogRecord};

/// You'll find three encoding configuration types that can be used
///     * [`EncodingConfig<E>`]
///     * [`EncodingConfigWithDefault<E>`]
///     * [`EncodingConfigFixed<E>`]
///

pub trait Encoder<T> {
    /// Encodes the input into the provided writer
    ///
    /// # Errors
    ///
    /// If an I/O error is encountered while encoding the input, an error variant will
    /// be returned.
    fn encode(&self, input: T, writer: &mut dyn io::Write) -> io::Result<usize>;
}

impl<E, T> Encoder<T> for Arc<E>
    where
        E: Encoder<T>,
{
    fn encode(&self, input: T, writer: &mut dyn Write) -> io::Result<usize> {
        (**self).encode(input, writer)
    }
}

pub trait MaybeAsLogMut {
    fn maybe_as_log_mut(&mut self) -> Option<&mut LogRecord>;
}

impl MaybeAsLogMut for Event {
    fn maybe_as_log_mut(&mut self) -> Option<&mut LogRecord> {
        match self {
            Event::Log(log) => Some(log),
            _ => None
        }
    }
}

/// The behavior of a encoding configuration
pub trait EncodingConfiguration {
    type Codec;
    // Required Accessors

    fn code(&self) -> &Self::Codec;
    fn schema(&self) -> &Option<String>;
    fn only_fields(&self) -> &Option<Vec<Vec<PathComponent>>>;
    fn except_fields(&self) -> &Option<Vec<String>>;

    /// Check that the configuration is valid.
    ///
    /// If an error is returned, the entire encoding configuration should be considered inoperable.
    ///
    /// For example, this checks if `except_fields` and `only_fields` items are mutually exclusive.
    fn validate(&self) -> Result<(), std::io::Error> {
        if let (Some(only_fields), Some(expect_fields)) = (&self.only_fields(), &self.expect_fields()) {
            if expect_fields.iter().any(|f| {
                let path_iter = PathIter::new(f).collect::<Vec<_>>();
                only_fields.iter().any(|v| v == &path_iter)
            }) {
                return Err("`expect_fields` and `only_fields` should be mutaually exclusive".into());
            }
        }

        Ok(())
    }

    /// Apply the EncodingConfig rules to the provided event.
    ///
    /// Currently, this is idempotent.
    fn apply_rules<T>(&self, event: &mut T)
        where
            T: MaybeAsLogMut
    {
        // No rules are currently applied to metrics
        if let Some(log) = event.maybe_as_log_mut() {
            // Ordering in here should not matter
            self.apply_except_fi
        }
    }
}