mod client;
mod config;
mod resource;
mod version;

pub use client::{Client, Error, WatchEvent, WatchParams};
pub use config::Config;
pub use resource::{ObjectList, ObjectMeta, Resource};
