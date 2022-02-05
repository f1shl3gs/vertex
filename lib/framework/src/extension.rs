use futures::future::BoxFuture;

pub type Extension = BoxFuture<'static, Result<(), ()>>;
