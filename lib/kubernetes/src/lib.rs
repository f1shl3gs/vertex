mod client;
mod config;
mod resource;
mod version;

pub use client::{Client, Error, ListParams, WatchEvent, WatchParams};
pub use resource::{ObjectList, ObjectMeta, Resource};
