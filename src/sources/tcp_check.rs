use std::time::Duration;
use std::time::Instant;

use configurable::configurable_component;
use event::metric::Metric;
use event::tags;
use framework::config::{OutputType, SourceConfig, SourceContext, default_interval};
use framework::{Pipeline, ShutdownSignal, Source};
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use tokio::net::TcpStream;

const fn default_timeout() -> Duration {
    Duration::from_secs(5)
}

/// This source creates stats by connecting to a TCP server.
#[configurable_component(source, name = "tcp_check")]
struct Config {
    /// Targets to connect
    targets: Vec<String>,

    /// Timeout for each check.
    #[serde(default = "default_timeout", with = "humanize::duration::serde")]
    timeout: Duration,

    /// Duration between each scrape.
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,
}

#[async_trait::async_trait]
#[typetag::serde(name = "tcp_check")]
impl SourceConfig for Config {
    async fn build(&self, ctx: SourceContext) -> crate::Result<Source> {
        if self.targets.is_empty() {
            return Err("`targets` is empty".into());
        }

        if self.timeout >= self.interval {
            return Err("`timeout` must be greater than or equal to `interval`".into());
        }

        let targets = self.targets.clone();
        let shutdown = ctx.shutdown;
        let output = ctx.output;
        let interval = self.interval;
        let timeout = self.timeout;

        Ok(Box::pin(async move {
            let mut tasks =
                FuturesUnordered::from_iter(targets.iter().map(|target| {
                    run(target, interval, timeout, output.clone(), shutdown.clone())
                }));

            while tasks.next().await.is_some() {}

            Ok(())
        }))
    }

    fn outputs(&self) -> Vec<OutputType> {
        vec![OutputType::metric()]
    }
}

async fn run(
    target: &str,
    interval: Duration,
    timeout: Duration,
    mut output: Pipeline,
    mut shutdown: ShutdownSignal,
) -> std::io::Result<()> {
    let start = crate::common::calculate_start(target, interval);
    let mut ticker = tokio::time::interval_at(start.into(), interval);

    let mut errors = 0i64;
    loop {
        tokio::select! {
            _ = ticker.tick() => {},
            _ = &mut shutdown => break,
        }

        let start = Instant::now();
        let (elapsed, status) = tokio::select! {
            _ = &mut shutdown => break,
            _ = tokio::time::sleep(timeout) => (start.elapsed(), false),
            result = TcpStream::connect(target) => (start.elapsed(), result.is_ok()),
        };

        if !status {
            errors += 1;
        }

        if let Err(_err) = output
            .send(vec![
                Metric::gauge_with_tags(
                    "tcp_check_duration_seconds",
                    "The total seconds of TCP connection",
                    elapsed,
                    tags!("target" => target),
                ),
                Metric::gauge_with_tags(
                    "tcp_check_status",
                    "Records errors occurring during TCP check",
                    status,
                    tags!("target" => target),
                ),
                Metric::sum_with_tags(
                    "tcp_check_connect_errors_total",
                    "The number of errors",
                    errors,
                    tags!("target" => target),
                ),
            ])
            .await
        {
            break;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>()
    }
}
