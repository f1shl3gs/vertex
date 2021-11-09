#[cfg(feature = "extensions-pprof")]
mod pprof;
mod exec;
#[cfg(feature = "extensions-jemalloc")]
mod jemalloc;

use futures::future::BoxFuture;

pub type Extension = BoxFuture<'static, Result<(), ()>>;