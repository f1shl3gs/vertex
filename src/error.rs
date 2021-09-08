use core::fmt::{Display};

pub trait Sealed {}

impl<T, E> Sealed for Result<T, E> where E: ext::StdError {}

impl<T> Sealed for Option<T> {}

pub trait Context<T, E>: Sealed {
    /// Wrap the error value with additional context
    fn context<C>(self, ctx: C) -> Result<T, Error>
        where
            C: Display + Send + Sync + 'static;

    /// Wrap the error value with additional context that is evaluated
    /// lazily only once an error does occur.
    fn with_context<C, F>(self, f: F) -> Result<T, Error>
        where
            C: Display + Send + Sync + 'static,
            F: FnOnce() -> C;
}

pub type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

mod ext {
    use super::*;

    pub trait StdError {
        fn ext_context<C>(self, ctx: C) -> Error
            where
                C: Display + Send + Sync + 'static;
    }
}

impl<T, E> Context<T, E> for Result<T, E>
    where
        E: ext::StdError + Send + Sync + 'static,
{
    fn context<C>(self, ctx: C) -> Result<T, Error> where C: Display + Send + Sync + 'static {
        self.map_err(|err| err.ext_context(ctx))
    }

    fn with_context<C, F>(self, f: F) -> Result<T, Error> where C: Display + Send + Sync + 'static, F: FnOnce() -> C {
        self.map_err(|err| err.ext_context(f()))
    }
}
