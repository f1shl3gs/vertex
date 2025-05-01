mod aggregate;
mod config;
pub mod preset;
mod serde_regex;

// Re-export
pub use aggregate::{LineAgg, Logic, Mode};
pub use config::{Config, Parser};
