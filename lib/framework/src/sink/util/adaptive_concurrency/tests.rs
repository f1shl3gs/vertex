#![allow(clippy::print_stderr)] //tests

use std::collections::VecDeque;
use std::fmt;
use std::fmt::Formatter;
use std::fs::{read_dir, File};
use std::io::Read;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use configurable::{configurable_component, Configurable};
use event::Event;
use futures::FutureExt;
use futures_util::future::BoxFuture;
use futures_util::{stream, SinkExt};
use rand::{thread_rng, Rng};
use rand_distr::Exp1;
use serde::{Deserialize, Serialize};
use testify::stats::{HistogramStats, LevelTimeHistogram, TimeHistogram, WeightedSumStats};
use tokio::sync::oneshot;
use tokio::time;
use tokio::time::sleep;
use tower::Service;

use crate::batch::{BatchSettings, EncodedEvent};
use crate::config::{DataType, SinkConfig, SinkContext};
use crate::sink::util::adaptive_concurrency::controller::ControllerStatistics;
use crate::sink::util::retries::RetryLogic;
use crate::sink::util::service::{Concurrency, RequestConfig};
use crate::sink::util::{EncodedLength, VecBuffer};
use crate::{Healthcheck, Sink};

#[derive(Configurable, Copy, Clone, Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
enum Action {
    /// Above the given limit, additional requests will return with an error
    #[default]
    Defer,

    /// Above the given limit, additional requests will be silently dropped
    Drop,
}

#[derive(Configurable, Copy, Clone, Debug, Default, Deserialize, Serialize)]
struct LimitParams {
    // The scale is the amount a request's dealy increases at higher levels of the variable.
    #[serde(default)]
    scale: f64,

    // The knee is the point above which a request's delay increase at an exponential scale
    // rather than a linear scale.
    knee_start: Option<usize>,

    knee_exp: Option<f64>,

    // The limit is the level above which more requests will be denied.
    limit: Option<usize>,

    // The action specifies how over-limit requests will be denied.
    #[serde(default)]
    action: Action,
}

impl LimitParams {
    fn action_at_level(&self, level: usize) -> Option<Action> {
        self.limit
            .and_then(|limit| (level > limit).then_some(self.action))
    }

    fn scale(&self, level: usize) -> f64 {
        ((level - 1) as f64).mul_add(
            self.scale,
            self.knee_start
                .map(|knee| {
                    self.knee_exp
                        .unwrap_or(self.scale + 1.0)
                        .powf(level.saturating_sub(knee) as f64)
                        - 1.0
                })
                .unwrap_or(0.0),
        )
    }
}

#[derive(Configurable, Copy, Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct TestParams {
    // The number of requests to issue.
    requests: usize,

    // The time interval between requests.
    #[serde(default = "default_interval")]
    interval: f64,

    // The delay is the base time every request takes return.
    delay: f64,

    // The jitter is the amount of per-request response time randomness, as a fraction of `delay`.
    // The average response time will be `delay * (1 + jitter)` and will have an exponential
    // distribution with λ=1.
    #[serde(default)]
    jitter: f64,

    #[serde(default)]
    concurrency_limit_params: LimitParams,

    #[serde(default)]
    rate: LimitParams,

    #[serde(default = "default_concurrency")]
    concurrency: Concurrency,
}

const fn default_interval() -> f64 {
    0.0
}

const fn default_concurrency() -> Concurrency {
    Concurrency::Adaptive
}

#[configurable_component(sink, name = "test")]
#[derive(Debug)]
struct TestConfig {
    request: RequestConfig,
    params: TestParams,

    // The statistics collected by running a test must be local to
    // that test and retained past the completion of the topology.
    // So, they are created by `Default` and may be cloned to retain
    // a handle.
    #[serde(skip)]
    control: Arc<Mutex<TestController>>,

    // Oh, the horror!
    #[serde(skip)]
    controller_stats: Arc<Mutex<Arc<Mutex<ControllerStatistics>>>>,
}

impl EncodedLength for Event {
    fn encoded_length(&self) -> usize {
        1
    }
}

#[async_trait::async_trait]
#[typetag::serde(name = "test")]
impl SinkConfig for TestConfig {
    async fn build(&self, cx: SinkContext) -> crate::Result<(Sink, Healthcheck)> {
        let mut batch_settings = BatchSettings::default();
        batch_settings.size.bytes = 9999;
        batch_settings.size.events = 1;
        batch_settings.timeout = Duration::from_secs(9999);

        let request = self.request.unwrap_with(&RequestConfig::default());
        let sink = request
            .batch_sink(
                TestRetryLogic,
                TestSink::new(self),
                VecBuffer::new(batch_settings.size),
                batch_settings.timeout,
                cx.acker(),
            )
            .with_flat_map(|event| stream::iter(Some(Ok(EncodedEvent::new(event, 0)))))
            .sink_map_err(|err| panic!("Fatal test sink error: {}", err));

        let healthcheck = futures::future::ok(()).boxed();

        // Dig deep to get at the internal controller statistics
        let stats = Arc::clone(
            &Pin::new(&sink.get_ref().get_ref().get_ref().get_ref())
                .get_ref()
                .controller
                .stats,
        );
        *self.controller_stats.lock().unwrap() = stats;

        Ok((crate::Sink::from_event_sink(sink), healthcheck))
    }

    fn input_type(&self) -> DataType {
        DataType::All
    }
}

#[derive(Clone, Debug)]
struct TestSink {
    control: Arc<Mutex<TestController>>,
    params: TestParams,
}

impl TestSink {
    fn new(config: &TestConfig) -> Self {
        Self {
            control: Arc::clone(&config.control),
            params: config.params,
        }
    }

    fn delay_at(&self, inflight: usize, rate: usize) -> f64 {
        self.params.delay
            * thread_rng().sample::<f64, _>(Exp1).mul_add(
                self.params.jitter,
                1.0 + self.params.concurrency_limit_params.scale(inflight)
                    + self.params.rate.scale(rate),
            )
    }
}

#[derive(Clone, Copy, Debug)]
enum Response {
    Ok,
}

impl crate::sink::util::sink::Response for Response {}

// The TestSink service doesn't actually do anything with the data, it
// just delays a while depending on the configured parameters and the
// yields a result
impl Service<Vec<Event>> for TestSink {
    type Response = Response;
    type Error = Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _req: Vec<Event>) -> Self::Future {
        let now = Instant::now();
        let mut control = self.control.lock().expect("Poisoned control lock");
        let stats = &mut control.stats;
        stats.start_request(now);
        let inflight = stats.inflight.level();
        let rate = stats.requests.len();

        let action = self
            .params
            .concurrency_limit_params
            .action_at_level(inflight)
            .or_else(|| self.params.rate.action_at_level(rate));

        match action {
            None => {
                let delay = self.delay_at(inflight, rate);
                respond_after(Ok(Response::Ok), delay, Arc::clone(&self.control))
            }

            Some(Action::Defer) => {
                let delay = self.delay_at(1, 1);
                respond_after(Err(Error::Deferred), delay, Arc::clone(&self.control))
            }

            Some(Action::Drop) => {
                control.end_request(now, false);
                Box::pin(std::future::pending())
            }
        }
    }
}

fn respond_after(
    resp: Result<Response, Error>,
    delay: f64,
    control: Arc<Mutex<TestController>>,
) -> BoxFuture<'static, Result<Response, Error>> {
    Box::pin(async move {
        sleep(Duration::from_secs_f64(delay)).await;

        let mut control = control.lock().expect("Poisoned control lock");
        control.end_request(Instant::now(), matches!(resp, Ok(Response::Ok)));

        resp
    })
}

#[derive(Clone, Copy, Debug, thiserror::Error)]
enum Error {
    #[error("deferred")]
    Deferred,
}

#[derive(Clone, Copy)]
struct TestRetryLogic;

impl RetryLogic for TestRetryLogic {
    type Error = Error;
    type Response = Response;

    fn is_retriable_error(&self, _err: &Self::Error) -> bool {
        true
    }
}

#[derive(Debug, Default)]
struct TestController {
    todo: usize,
    send_done: Option<oneshot::Sender<()>>,
    stats: Statistics,
}

impl TestController {
    /*    fn new(todo: usize, send_done: oneshot::Sender<()>) -> Self {
        Self {
            todo,
            send_done: Some(send_done),
            stats: Default::default(),
        }
    }*/

    fn end_request(&mut self, now: Instant, completed: bool) {
        self.stats.end_request(now, completed);
        if self.stats.completed >= self.todo {
            if let Some(done) = self.send_done.take() {
                done.send(()).expect("Could not send done signal");
            }
        }
    }
}

#[derive(Default)]
struct Statistics {
    completed: usize,
    inflight: LevelTimeHistogram,
    rate: TimeHistogram,
    requests: VecDeque<Instant>,
}

impl fmt::Debug for Statistics {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("SharedData")
            .field("completed", &self.completed)
            .field("inflight", &self.inflight)
            .field("rate", &self.rate)
            .field("requests", &self.requests.len())
            .finish()
    }
}

impl Statistics {
    fn start_request(&mut self, now: Instant) {
        self.prune_old_requests(now);
        self.requests.push_back(now);
        self.rate.add(self.requests.len(), now);
        self.inflight.adjust(1, now);
    }

    fn end_request(&mut self, now: Instant, completed: bool) {
        self.prune_old_requests(now);
        self.rate.add(self.requests.len(), now);
        self.inflight.adjust(-1, now);
        self.completed += completed as usize
    }

    /// Prune any requests that are more than one second old. The
    /// `requests` deque is used to track the rate at which requests
    /// are being issued. As such, it needs to be pruned of old
    /// requests any time a request status changes. Since all requests
    /// are inserted in chronological order, this function simply looks
    /// at the head of the deque and pops off all entries that are more
    /// than one second old. In this way, the length is always equal to
    /// the number of requests per second.
    fn prune_old_requests(&mut self, now: Instant) {
        let then = now - Duration::from_secs(1);

        while let Some(&first) = self.requests.get(0) {
            if first > then {
                break;
            }

            self.requests.pop_front();
        }
    }
}

#[derive(Debug)]
struct TestResults {
    stats: Statistics,
    cstats: ControllerStatistics,
}

async fn run_test(_params: TestParams) -> TestResults {
    todo!("implement")

    /*let (send_done, is_done) = oneshot::channel();

    let test_config = TestConfig {
        request: RequestConfig {
            concurrency: params.concurrency,
            rate_limit_num: Some(9999),
            timeout: Some(Duration::from_secs(1)),
            ..Default::default()
        },
        params,
        control: Arc::new(Mutex::new(TestController::new(params.requests, send_done))),
        controller_stats: Default::default(),
    };

    let control = Arc::clone(&test_config.control);
    let cstats = Arc::clone(&test_config.controller_stats);

    let mut config = config::Config::builder();
    let mock_logs = mock::MockLogsConfig {
        lines: vec!["some line".into()],
        count: params.requests,
        interval: Duration::from_secs(params.interval as u64),
    };
    config.add_source("in", mock_logs);
    config.add_sink("out", &["in"], test_config);

    let (topology, _crash) =
        crate::topology::test::start_topology(config.build().unwrap(), false).await;

    is_done.await.expect("Test failed to complete");
    topology.stop().await;

    let control = Arc::try_unwrap(control)
        .expect("Failed to unwrap control Arc")
        .into_inner()
        .expect("Failed to unwrap control Mutex");
    let stats = control.stats;

    let cstats = Arc::try_unwrap(cstats)
        .expect("Failed to unwrap controller_stats Arc")
        .into_inner()
        .expect("Failed to unwrap controller_stats Mutex");
    let cstats = Arc::try_unwrap(cstats)
        .expect("Failed to unwrap controller_stats Arc")
        .into_inner()
        .expect("Failed to unwrap controller_stats Mutex");

    let metrics = controller
        .capture_metrics()
        .into_iter()
        .map(|metric| (metric.name().to_string(), metric))
        .collect::<HashMap<_, _>>();

    // Ensure basic statistics are captured, don't actually examine them
    assert!(matches!(
        metrics
            .get("adaptive_concurrency_observed_rtt")
            .unwrap()
            .value(),
        &MetricValue::Histogram { .. }
    ));
    assert!(matches!(
        metrics
            .get("adaptive_concurrency_averaged_rtt")
            .unwrap()
            .value(),
        &MetricValue::Histogram { .. }
    ));
    if params.concurrency == Concurrency::Adaptive {
        assert!(matches!(
            metrics.get("adaptive_concurrency_limit").unwrap().value(),
            &MetricValue::Histogram { .. }
        ));
    }
    assert!(matches!(
        metrics
            .get("adaptive_concurrency_inflight")
            .unwrap()
            .value(),
        &MetricValue::Histogram { .. }
    ));

    TestResults { stats, cstats }*/
}

mod mock {
    use crate::config::{DataType, Output, SourceConfig, SourceContext};
    use crate::{Pipeline, ShutdownSignal, Source};
    use chrono::Utc;
    use configurable::configurable_component;
    use event::LogRecord;
    use log_schema::log_schema;
    use std::task::Poll;
    use std::time::Duration;

    #[configurable_component(source, name = "mock_logs")]
    #[derive(Debug)]
    pub struct MockLogsConfig {
        pub lines: Vec<String>,
        pub count: usize,
        pub interval: Duration,
    }

    #[async_trait::async_trait]
    #[typetag::serde(name = "mock_logs")]
    impl SourceConfig for MockLogsConfig {
        async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
            Ok(Box::pin(mock_logs_source(
                self.interval,
                self.count,
                self.lines.clone(),
                cx.shutdown,
                cx.output,
            )))
        }

        fn outputs(&self) -> Vec<Output> {
            vec![Output::default(DataType::Log)]
        }
    }

    async fn mock_logs_source(
        interval: Duration,
        count: usize,
        lines: Vec<String>,
        mut shutdown: ShutdownSignal,
        mut output: Pipeline,
    ) -> Result<(), ()> {
        let mut interval = (!interval.is_zero())
            .then_some(interval)
            .map(tokio::time::interval);

        for _i in 0..count {
            if matches!(futures::poll!(&mut shutdown), Poll::Ready(_)) {
                break;
            }

            if let Some(interval) = &mut interval {
                interval.tick().await;
            }

            let now = Utc::now();
            let logs = lines
                .iter()
                .map(|line| {
                    let mut log = LogRecord::from(line.to_string());
                    log.insert_field(log_schema().timestamp_key(), now);
                    log
                })
                .collect::<Vec<_>>();

            if let Err(err) = output.send(logs).await {
                error!(message = "Error sending logs", ?err);
                return Err(());
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
enum FailureMode {
    ExceededMinimum,
    ExceededMaximum,
}

#[derive(Debug)]
struct Failure {
    stat_name: String,
    mode: FailureMode,
    value: f64,
    reference: f64,
}

#[derive(Clone, Copy, Debug, Deserialize)]
struct Range(f64, f64);

impl Range {
    fn assert_usize(&self, value: usize, name1: &str, name2: &str) -> Option<Failure> {
        if value < self.0 as usize {
            Some(Failure {
                stat_name: format!("{} {}", name1, name2),
                mode: FailureMode::ExceededMinimum,
                value: value as f64,
                reference: self.0,
            })
        } else if value > self.1 as usize {
            Some(Failure {
                stat_name: format!("{} {}", name1, name2),
                mode: FailureMode::ExceededMaximum,
                value: value as f64,
                reference: self.1,
            })
        } else {
            None
        }
    }

    fn assert_f64(&self, value: f64, name1: &str, name2: &str) -> Option<Failure> {
        if value < self.0 {
            Some(Failure {
                stat_name: format!("{} {}", name1, name2),
                mode: FailureMode::ExceededMinimum,
                value,
                reference: self.0,
            })
        } else if value > self.1 {
            Some(Failure {
                stat_name: format!("{} {}", name1, name2),
                mode: FailureMode::ExceededMaximum,
                value,
                reference: self.1,
            })
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
struct ResultTest {
    min: Option<Range>,
    max: Option<Range>,
    mode: Option<Range>,
    mean: Option<Range>,
}

impl ResultTest {
    fn compare_histogram(&self, stat: HistogramStats, name: &str) -> Vec<Failure> {
        vec![
            self.min
                .and_then(|range| range.assert_usize(stat.min, name, "min")),
            self.max
                .and_then(|range| range.assert_usize(stat.max, name, "max")),
            self.mean
                .and_then(|range| range.assert_f64(stat.mean, name, "mean")),
            self.mode
                .and_then(|range| range.assert_usize(stat.mode, name, "mode")),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
    }

    fn compare_weighted_sum(&self, stat: WeightedSumStats, name: &str) -> Vec<Failure> {
        vec![
            self.min
                .and_then(|range| range.assert_f64(stat.min, name, "min")),
            self.max
                .and_then(|range| range.assert_f64(stat.max, name, "max")),
            self.mean
                .and_then(|range| range.assert_f64(stat.mean, name, "mean")),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
    }
}

#[derive(Debug, Deserialize)]
struct ControllerResults {
    observed_rtt: Option<ResultTest>,
    averaged_rtt: Option<ResultTest>,
    concurrency_limit: Option<ResultTest>,
    inflight: Option<ResultTest>,
}

#[derive(Debug, Deserialize)]
struct StatsResults {
    inflight: Option<ResultTest>,
    rate: Option<ResultTest>,
}

#[derive(Debug, Deserialize)]
struct TestInput {
    params: TestParams,
    stats: StatsResults,
    controller: ControllerResults,
}

async fn run_compare(file_path: PathBuf, input: TestInput) {
    eprintln!("Running test in {:?}", file_path);

    let results = run_test(input.params).await;
    let mut failures = Vec::new();

    if let Some(test) = input.stats.inflight {
        let inflight = results.stats.inflight.stats().unwrap();
        failures.extend(test.compare_histogram(inflight, "stats inflight"));
    }

    if let Some(test) = input.stats.rate {
        let rate = results.stats.rate.stats().unwrap();
        failures.extend(test.compare_histogram(rate, "stats rate"));
    }

    if let Some(test) = input.controller.inflight {
        let inflight = results.cstats.inflight.stats().unwrap();
        failures.extend(test.compare_histogram(inflight, "controller inflight"));
    }

    if let Some(test) = input.controller.concurrency_limit {
        let concurrency_limit = results.cstats.concurrency_limit.stats().unwrap();
        failures.extend(test.compare_histogram(concurrency_limit, "controller concurrency_limit"));
    }

    if let Some(test) = input.controller.observed_rtt {
        let observed_rtt = results.cstats.observed_rtt.stats().unwrap();
        failures.extend(test.compare_weighted_sum(observed_rtt, "controller observed_rtt"));
    }

    if let Some(test) = input.controller.averaged_rtt {
        let averaged_rtt = results.cstats.averaged_rtt.stats().unwrap();
        failures.extend(test.compare_weighted_sum(averaged_rtt, "controller averaged_rtt"));
    }

    for failure in &failures {
        let mode = match failure.mode {
            FailureMode::ExceededMaximum => "maximum",
            FailureMode::ExceededMinimum => "minimum",
        };

        eprintln!(
            "Comparison failed: {} = {}; {} = {}",
            failure.stat_name, failure.value, mode, failure.reference
        );
    }

    assert!(failures.is_empty(), "{:#?}", results)
}

// TODO: enable this
#[ignore]
#[tokio::test]
async fn all_tests() {
    const PATH: &str = "tests/fixtures/adaptive-concurrency";

    // Read and parse everything first
    let mut entries = read_dir(PATH)
        .expect("Could not open data directory")
        .map(|entry| entry.expect("Could not read data directory").path())
        .filter_map(|file_path| {
            if (file_path.extension().map(|ext| ext == "yaml")).unwrap_or(false) {
                let mut data = String::new();
                File::open(&file_path)
                    .unwrap()
                    .read_to_string(&mut data)
                    .unwrap();
                let input: TestInput = serde_yaml::from_str(&data)
                    .unwrap_or_else(|err| panic!("Invalid YAML in {:?}: {:?}", file_path, err));

                Some((file_path, input))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    entries.sort_unstable_by_key(|entry| entry.0.to_string_lossy().to_string());

    time::pause();

    // The first delay takes just slightly longer than all the rest,
    // which causes the first test to run differently than all the
    // others. Throw in a dummy delay to take up this delay "slack"
    sleep(Duration::from_millis(1)).await;
    time::advance(Duration::from_millis(1)).await;

    // Then run all the tests
    for (file, input) in entries {
        run_compare(file, input).await
    }
}
