#[derive(Debug)]
pub enum FramingError {
    Io(std::io::Error),
}

impl From<std::io::Error> for FramingError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}
