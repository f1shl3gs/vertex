mod client;
mod config;
pub mod resource;
mod version;

pub use client::{Client, Error, WatchEvent, WatchParams};
pub use config::Config;
