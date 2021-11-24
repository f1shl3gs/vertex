#[macro_use]
extern crate tracing;

use tracing::Dispatch;
use tracing_limit::RateLimitedLayer;
use tracing_subscriber::layer::SubscriberExt;

fn main() {
    let subscriber = tracing_subscriber::registry::Registry::default()
        .with(RateLimitedLayer::new(
            tracing_subscriber::fmt::Layer::default().without_time(),
        ))
        .with(tracing_subscriber::filter::EnvFilter::from("trace"));

    let dispatch = Dispatch::new(subscriber);

    tracing::dispatcher::with_default(&dispatch, || {
        for i in 0..40 {
            trace!("This field is not rate limited!");
            info!(
                message = "This message is rate limited",
                count = &i,
                internal_log_rate_secs = 5,
            );
            std::thread::sleep(std::time::Duration::from_millis(1000));
        }
    })
}
