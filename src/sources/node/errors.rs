use std::{
    borrow::Cow,
    error,
    fmt::{self, Formatter},
    io, num,
    path::PathBuf,
    string::ParseError,
};

#[derive(Debug)]
pub enum Context {
    File { path: PathBuf },
    Message { text: Cow<'static, str> },
}

impl From<PathBuf> for Context {
    fn from(path: PathBuf) -> Self {
        Self::File { path }
    }
}

#[macro_export]
macro_rules! invalid_error {
    ($($arg:tt)*) => {{
        let msg = std::fmt::format(format_args!($($arg)*));
        Err(Error::new_invalid(msg))
    }}
}

/// Error type for data fetching operations
///
/// Errors are originated from the underlying OS, data parsing
/// or FFI call errors, and it should be assumed that this error
/// is unrecoverable and data can't be fetched at all.
///
/// Note: users **should not** rely on any internal API of this struct,
/// as it is a subject of change in any moment.
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
    pub const fn new(source: io::Error, context: Context) -> Self {
        Self {
            source,
            context: Some(context),
        }
    }

    #[inline]
    pub fn is_not_found(&self) -> bool {
        self.source.kind() == io::ErrorKind::NotFound
    }

    pub fn new_invalid<T>(msg: T) -> Self
    where
        T: Into<Cow<'static, str>>,
    {
        Self {
            source: io::Error::from(io::ErrorKind::InvalidData),
            context: Some(Context::Message { text: msg.into() }),
        }
    }

    /// Returns error representing last OS error that occurred.
    ///
    /// This method is considered to be an internal API
    /// and should not be used by external parties.
    pub fn last_os_error() -> Self {
        Self {
            source: io::Error::last_os_error(),
            #[cfg(feature = "backtrace")]
            backtrace: Some(Backtrace::new()),
            context: None,
        }
    }

    // pub fn from_context<C, E>(ctx: C, err: E) -> Self
    //     where
    //         C: Display + Send + Sync + 'static,
    //         E: ext::StdError + Send + Sync + 'static,
    // {
    //     Self {
    //         source: err,
    //         context: Some(ctx),
    //     }
    // }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.context {
            Some(Context::File { ref path }) => f.write_fmt(format_args!(
                "Unable to parse {}, unsupported format",
                path.display()
            )),

            Some(Context::Message { text }) => f.write_str(text.as_ref()),

            None => return fmt::Display::fmt(&self.source, f),
        }?;

        f.write_str(": ")?;
        fmt::Display::fmt(&self.source, f)
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

impl From<ParseError> for Error {
    fn from(err: ParseError) -> Self {
        let err = io::Error::new(io::ErrorKind::InvalidInput, err);

        Self {
            source: err,
            context: None,
        }
    }
}

impl From<glob::PatternError> for Error {
    fn from(err: glob::PatternError) -> Self {
        let err = io::Error::new(io::ErrorKind::InvalidData, err);

        Self {
            source: err,
            context: None,
        }
    }
}

pub trait ErrorContext<T, E>
where
    E: Into<Error>,
{
    /// Adds some context to the error
    fn context<C: ToString>(self, ctx: C) -> Result<T, Error>;

    /// Adds context to the error, evaluating the context function only if there
    /// is an Err
    fn with_context<S: ToString, F: FnOnce() -> S>(self, f: F) -> Result<T, Error>;
}

impl<T, E> ErrorContext<T, E> for Result<T, E>
where
    E: Into<Error>,
{
    fn context<C: ToString>(self, ctx: C) -> Result<T, Error> {
        self.map_err(|err| {
            let mut err = err.into();
            err.context = Some(Context::Message {
                text: ctx.to_string().into(),
            });

            err
        })
    }

    fn with_context<S: ToString, F: FnOnce() -> S>(self, f: F) -> Result<T, Error> {
        self.map_err(|err| {
            let mut err = err.into();
            let ctx = f().to_string();

            err.context = Some(Context::Message { text: ctx.into() });

            err
        })
    }
}
