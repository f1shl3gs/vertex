use std::fmt;

use event::Events;
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct ClosedError;

impl fmt::Display for ClosedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Sender is closed.")
    }
}

impl std::error::Error for ClosedError {}

impl From<mpsc::error::SendError<Events>> for ClosedError {
    fn from(_: mpsc::error::SendError<Events>) -> Self {
        Self
    }
}
