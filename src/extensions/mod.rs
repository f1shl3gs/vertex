#[cfg(feature = "extension-pprof")]
mod pprof;
mod exec;
#[cfg(feature = "extension-jemalloc")]
mod jemalloc;

use futures::future::BoxFuture;

pub type Extension = BoxFuture<'static, Result<(), ()>>;