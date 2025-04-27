pub mod http;
pub mod tcp;
pub mod udp;
pub mod unix;

pub type Source = futures::future::BoxFuture<'static, Result<(), ()>>;
