pub mod http;
pub mod tcp;
pub mod unix;

pub type Source = futures::future::BoxFuture<'static, Result<(), ()>>;
