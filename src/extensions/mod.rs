#[cfg(feature = "extension-pprof")]
mod pprof;
mod exec;

use futures::future::BoxFuture;

pub type Extension = BoxFuture<'static, Result<(), ()>>;