pub mod decoding;
pub mod encoding;

pub type Error = Box<dyn std::error::Error + Send + Sync + 'static>;
