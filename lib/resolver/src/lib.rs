// mod cache;
mod config;
mod proto;
mod resolver;
mod singleflight;

pub use config::{Config, Hosts};
pub use proto::{Record, RecordClass, RecordData, RecordType};
pub use resolver::{Error, Lookup, LookupIntoIter, Resolver};
