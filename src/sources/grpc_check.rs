use std::time::{Duration, Instant};

use async_trait::async_trait;
use configurable::configurable_component;
use event::{Metric, tags};
use framework::config::{Output, SourceConfig, SourceContext, default_interval};
use framework::{Pipeline, ShutdownSignal, Source};
use futures::{StreamExt, stream::FuturesUnordered};
use tonic::Code;
use tonic_health::pb::HealthCheckRequest;
use tonic_health::pb::health_check_response::ServingStatus;
use tonic_health::pb::health_client::HealthClient;

const fn default_timeout() -> Duration {
    Duration::from_secs(5)
}

/// gRPC check the grpc service and produce metrics.
///
/// https://github.com/grpc/grpc/blob/master/doc/health-checking.md
#[configurable_component(source, name = "grpc_check")]
struct Config {
    /// The service name to query for health status.
    #[configurable(required, example = "grpc.health.v1.Health")]
    service: String,

    /// Endpoint for gRPC service.
    #[configurable(required)]
    targets: Vec<String>,

    /// This sources collects metrics on an interval.
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,

    /// Timeout for gRPC request, it's value should be less than `interval`.
    #[serde(default = "default_timeout", with = "humanize::duration::serde")]
    timeout: Duration,
}

#[async_trait]
#[typetag::serde(name = "grpc_check")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> framework::Result<Source> {
        Ok(Box::pin(run(
            self.service.clone(),
            self.targets.clone(),
            self.timeout,
            self.interval,
            cx.output,
            cx.shutdown,
        )))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::metrics()]
    }

    fn can_acknowledge(&self) -> bool {
        false
    }
}

async fn run(
    service: String,
    targets: Vec<String>,
    timeout: Duration,
    interval: Duration,
    output: Pipeline,
    shutdown: ShutdownSignal,
) -> Result<(), ()> {
    let tasks = FuturesUnordered::new();

    for target in targets {
        let mut output = output.clone();
        let mut shutdown = shutdown.clone();
        let service = service.clone();

        tasks.push(tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);

            loop {
                tokio::select! {
                    biased;

                    _ = &mut shutdown => break,
                    _ = ticker.tick() => {}
                }

                let metrics = scrape(&service, &target, timeout).await;

                if let Err(err) = output.send(metrics).await {
                    warn!(message = "send metrics failed", %err);
                    break;
                }
            }
        }))
    }

    let _ = tasks.collect::<Vec<_>>().await;

    Ok(())
}

async fn scrape(service: &str, target: &str, timeout: Duration) -> Vec<Metric> {
    let start = Instant::now();
    let result = match tokio::time::timeout(timeout, check(service, target.to_string())).await {
        Ok(result) => result,
        Err(err) => Err(err.into()),
    };
    let elapsed = start.elapsed();

    let tags = tags!(
        "service" => service,
        "endpoint" => target
    );

    let (code, serving_status) = result.unwrap_or_else(|err| {
        warn!(message = "check gRPC service failed", %err);

        (Code::Unknown, ServingStatus::Unknown)
    });
    let metrics = vec![
        Metric::gauge_with_tags(
            "grpc_check_duration_seconds",
            "Duration of gRPC request",
            elapsed,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "grpc_check_status_code",
            "Response gRPC status code",
            i32::from(code),
            tags,
        ),
        Metric::gauge_with_tags(
            "grpc_check_healthcheck_response",
            "HealthCheck response",
            i32::from(serving_status),
            tags!(
                "service" => service,
                "endpoint" => target,
                "serving_status" => serving_status.as_str_name()
            ),
        ),
    ];

    metrics
}

async fn check(service: &str, address: String) -> framework::Result<(Code, ServingStatus)> {
    let conn = tonic::transport::Endpoint::new(address)?.connect().await?;
    let mut client = HealthClient::new(conn);
    let result = client
        .check(HealthCheckRequest {
            service: service.to_string(),
        })
        .await;

    Ok(match result {
        Ok(resp) => (Code::Ok, resp.into_inner().status()),
        Err(err) => (err.code(), ServingStatus::Unknown),
    })
}

#[cfg(test)]
mod tests {
    use event::MetricValue::Gauge;
    use tonic::transport::Server;

    use super::*;
    use crate::testing::trace_init;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>();
    }

    #[tokio::test]
    async fn check() {
        trace_init();

        mod mock {
            use std::task::{Context, Poll};

            use http::Request;
            use tonic::body::Body;
            use tonic::codegen::BoxFuture;
            use tonic::server::NamedService;
            use tower::Service;

            #[derive(Clone)]
            pub struct DummyService {}

            impl NamedService for DummyService {
                const NAME: &'static str = "dummy";
            }

            impl Service<Request<Body>> for DummyService {
                type Response = http::Response<Body>;
                type Error = std::convert::Infallible;
                type Future = BoxFuture<Self::Response, Self::Error>;

                fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
                    Poll::Ready(Ok(()))
                }

                fn call(&mut self, _req: Request<Body>) -> Self::Future {
                    todo!()
                }
            }
        }

        use mock::DummyService;
        use tonic::server::NamedService;

        let (health_reporter, health_service) = tonic_health::server::health_reporter();
        let addr = testify::next_addr();
        let dummy_service = DummyService {};
        let endpoint = format!("http://{addr}");

        // server not start
        let metrics = scrape(
            DummyService::NAME,
            endpoint.as_str(),
            Duration::from_secs(1),
        )
        .await;
        assert_eq!(metrics.len(), 3);
        assert_eq!(metrics[1].value, Gauge(2.0)); // grpc response code, 2 for Code::Unknown
        assert_eq!(metrics[2].value, Gauge(0.0)); // healthcheck serving status
        assert_eq!(
            metrics[2].tag_value("serving_status").unwrap().to_string(),
            ServingStatus::Unknown.as_str_name()
        );

        tokio::spawn(
            Server::builder()
                .add_service(dummy_service)
                .add_service(health_service)
                .serve(addr),
        );

        // wait for grpc service startup
        tokio::time::sleep(Duration::from_secs(1)).await;

        health_reporter
            .set_service_status("dummy", tonic_health::ServingStatus::Serving)
            .await;
        let metrics = scrape(
            DummyService::NAME,
            endpoint.as_str(),
            Duration::from_secs(1),
        )
        .await;
        assert_eq!(metrics.len(), 3);
        assert_eq!(metrics[1].value, Gauge(0.0)); // grpc response code, 0 for Code::Ok
        assert_eq!(metrics[2].value, Gauge(1.0)); // healthcheck serving status
        assert_eq!(
            metrics[2].tag_value("serving_status").unwrap().to_string(),
            ServingStatus::Serving.as_str_name()
        );

        health_reporter
            .set_service_status("dummy", tonic_health::ServingStatus::NotServing)
            .await;
        let metrics = scrape(
            DummyService::NAME,
            endpoint.as_str(),
            Duration::from_secs(1),
        )
        .await;
        assert_eq!(metrics.len(), 3);
        assert_eq!(metrics[1].value, Gauge(0.0)); // grpc response code, 0 for Code::Ok
        assert_eq!(metrics[2].value, Gauge(2.0)); // healthcheck serving status
        assert_eq!(
            metrics[2].tag_value("serving_status").unwrap().to_string(),
            ServingStatus::NotServing.as_str_name()
        );
    }
}
