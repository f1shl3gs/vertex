use std::{
    error,
    io,
    fmt::{self, Formatter},
    borrow::Cow,
    num,
    path::{PathBuf},
    string::ParseError,
};
use slog::{Record, Key, Serializer};

/// A specialized Result type for `gathering` functions.
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Context {
    File {
        path: PathBuf
    },
    Message {
        text: Cow<'static, str>,
    },
}

#[derive(Debug)]
pub struct Error {
    source: io::Error,
    context: Option<Context>,
}

impl Error {
    /// Create new `Error` instance from `io::Error` and context details
    ///
    /// This method is considered to be an internal API
    /// and should not be used by external parties
    pub fn new(source: io::Error, context: Context) -> Self {
        Self {
            source,
            context: Some(context),
        }
    }

    /// Replace error context with `Context::Message` instance
    pub fn with_message<T>(mut self, text: T) -> Self
        where
            T: Into<Cow<'static, str>>
    {
        self.context = Some(Context::Message { text: text.into() });

        self
    }

    pub fn is_not_found(&self) -> bool {
        self.source.kind() == io::ErrorKind::NotFound
    }

    pub fn new_invalid_with_message<T>(msg: T) -> Self
        where
            T: Into<Cow<'static, str>>
    {
        Self {
            source: io::Error::from(io::ErrorKind::InvalidData),
            context: Some(Context::Message { text: msg.into() }),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.context {
            Some(Context::File { ref path }) => {
                f.write_fmt(format_args!("Unable to parse {}, unsupported format", path.display()))
            }

            Some(Context::Message { text }) => f.write_str(text.as_ref()),

            None => return fmt::Display::fmt(&self.source, f)
        }?;

        f.write_str(": ")?;
        fmt::Display::fmt(&self.source, f)
    }
}

impl slog::Value for Error {
    fn serialize(&self, _record: &Record, key: Key, serializer: &mut dyn Serializer) -> slog::Result {
        serializer.emit_str(key, &self.to_string())
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(&self.source)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self {
            source: err,
            context: None,
        }
    }
}

impl From<num::ParseIntError> for Error {
    fn from(err: num::ParseIntError) -> Self {
        let inner = io::Error::new(io::ErrorKind::InvalidData, err);
        Self::from(inner)
    }
}

impl From<num::ParseFloatError> for Error {
    fn from(err: num::ParseFloatError) -> Self {
        let inner = io::Error::new(io::ErrorKind::InvalidData, err);
        Self::from(inner)
    }
}

impl From<std::string::ParseError> for Error {
    fn from(err: ParseError) -> Self {
        let err = io::Error::new(io::ErrorKind::InvalidInput, err);

        Self {
            source: err,
            context: None,
        }
    }
}

