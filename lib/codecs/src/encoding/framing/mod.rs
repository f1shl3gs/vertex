use std::io::Error;

mod character_delimited;

#[derive(Debug)]
pub enum FramingError {
    Io(Error),
}

impl From<Error> for FramingError {
    fn from(err: Error) -> Self {
        Self::Io(err)
    }
}
