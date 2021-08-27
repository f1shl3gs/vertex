use futures::future::BoxFuture;

mod pprof;
mod exec;

pub type Extension = BoxFuture<'static, Result<(), ()>>;