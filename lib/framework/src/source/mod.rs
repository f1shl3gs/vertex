mod udp;
pub mod util;

use futures::future::BoxFuture;

pub use udp::UdpSource;

pub type Source = BoxFuture<'static, Result<(), ()>>;
