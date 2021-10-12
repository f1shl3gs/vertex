mod settings;
mod maybe_tls;
mod incoming;

pub use settings::{TLSConfig, MaybeTLSSettings};
pub use maybe_tls::{
    MaybeTLS,
};

use std::path::PathBuf;
use snafu::Snafu;

#[derive(Debug, Snafu)]
pub enum TLSError {
    #[snafu(display("Could not open {} file {:?}: {}", note, filename, source))]
    FileOpenFailed {
        note: &'static str,
        filename: PathBuf,
        source: std::io::Error,
    },

    #[snafu(display("Incoming listener failed: {}", source))]
    IncomingListener { source: tokio::io::Error}
}