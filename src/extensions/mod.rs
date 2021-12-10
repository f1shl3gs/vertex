mod exec;
#[cfg(feature = "extensions-jemalloc")]
mod jemalloc;
#[cfg(feature = "extensions-pprof")]
mod pprof;

use futures::future::BoxFuture;

pub type Extension = BoxFuture<'static, Result<(), ()>>;
