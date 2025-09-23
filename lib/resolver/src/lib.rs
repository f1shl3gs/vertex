// mod cache;
mod config;
#[cfg(target_family = "unix")]
mod hosts;
mod proto;
mod resolver;
mod singleflight;

pub use config::Config;
pub use proto::{Record, RecordClass, RecordData, RecordType};
pub use resolver::{Error, Lookup, LookupIntoIter, Resolver};
