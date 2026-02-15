use std::borrow::Cow;
use std::cmp;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, ready};
use std::time::Duration;

use futures::FutureExt;
use tokio::time::{Sleep, sleep};
use tower::retry::Policy;
use tower::timeout::error::Elapsed;

use crate::Error;

pub enum RetryAction {
    /// Indicate that this request should be retried with a reason
    Retry(Cow<'static, str>),
    /// Indicate that this request should not be retried with a reason
    DontRetry(Cow<'static, str>),
    /// Indicate that this request should not be retried but the request was successful
    Successful,
}

pub trait RetryLogic: Clone + Send + Sync + 'static {
    type Error: std::error::Error + Send + Sync + 'static;
    type Response;

    fn is_retriable_error(&self, err: &Self::Error) -> bool;

    fn should_retry_resp(&self, _response: &Self::Response) -> RetryAction {
        // Treat the default as the request is successful
        RetryAction::Successful
    }
}

#[derive(Debug, Clone)]
pub struct FixedRetryPolicy<L> {
    remaining_attempts: usize,
    previous_duration: Duration,
    current_duration: Duration,
    max_duration: Duration,
    logic: L,
}

pub struct RetryPolicyFuture {
    delay: Pin<Box<Sleep>>,
}

impl<L: RetryLogic> FixedRetryPolicy<L> {
    pub fn new(
        remaining_attempts: usize,
        initial_backoff: Duration,
        max_duration: Duration,
        logic: L,
    ) -> Self {
        FixedRetryPolicy {
            remaining_attempts,
            previous_duration: Duration::from_secs(0),
            current_duration: initial_backoff,
            max_duration,
            logic,
        }
    }

    fn advance(&mut self) {
        let next_duration = self.previous_duration + self.current_duration;

        self.remaining_attempts -= 1;
        self.previous_duration = self.current_duration;
        self.current_duration = cmp::min(next_duration, self.max_duration);
    }

    fn backoff(&self) -> Duration {
        self.current_duration
    }

    fn build_retry(&mut self) -> RetryPolicyFuture {
        self.advance();
        let delay = Box::pin(sleep(self.backoff()));

        debug!(message = "Retrying request", delay_ms = %self.backoff().as_millis());
        RetryPolicyFuture { delay }
    }
}

impl<Req, Res, L> Policy<Req, Res, Error> for FixedRetryPolicy<L>
where
    Req: Clone,
    L: RetryLogic<Response = Res>,
{
    type Future = RetryPolicyFuture;

    fn retry(&mut self, _: &mut Req, result: &mut Result<Res, Error>) -> Option<Self::Future> {
        match result {
            Ok(response) => match self.logic.should_retry_resp(response) {
                RetryAction::Retry(reason) => {
                    if self.remaining_attempts == 0 {
                        error!(
                            message =
                                "OK/retry response but retries exhausted; dropping the request",
                            ?reason
                        );
                        return None;
                    }

                    warn!(message = "Retrying after response", %reason);

                    Some(self.build_retry())
                }

                RetryAction::DontRetry(reason) => {
                    error!(message = "Not retriable; dropping the request", ?reason);
                    None
                }

                RetryAction::Successful => None,
            },
            Err(err) => {
                if self.remaining_attempts == 0 {
                    error!(message = "Retries exhausted; dropping the request", %err);
                    return None;
                }

                if let Some(expected) = err.downcast_ref::<L::Error>() {
                    if self.logic.is_retriable_error(expected) {
                        warn!(message = "Retrying after error", error = %expected);
                        Some(self.build_retry())
                    } else {
                        error!(
                            message = "Non-retriable error; dropping the request",
                            %err
                        );
                        None
                    }
                } else if err.downcast_ref::<Elapsed>().is_some() {
                    warn!(
                        message = "Request timed out. If this happens often while the events are actually reaching their destination, try decreasing `batch.max_bytes` and/or using `compression` if applicable. Alternatively `request.timeout_secs` can be increased."
                    );

                    Some(self.build_retry())
                } else {
                    error!(
                        message = "Unexpected error type; dropping the request",
                        %err
                    );
                    None
                }
            }
        }
    }

    fn clone_request(&mut self, request: &Req) -> Option<Req> {
        Some(request.clone())
    }
}

// Safety: `L` is never pinned and we use no unsafe pin projections
// therefore this safe.
impl Unpin for RetryPolicyFuture {}

impl Future for RetryPolicyFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        ready!(self.delay.poll_unpin(cx));
        Poll::Ready(())
    }
}

impl RetryAction {
    pub const fn is_retryable(&self) -> bool {
        matches!(self, RetryAction::Retry(_))
    }

    pub const fn is_not_retryable(&self) -> bool {
        matches!(self, RetryAction::DontRetry(_))
    }

    pub const fn is_successful(&self) -> bool {
        matches!(self, RetryAction::Successful)
    }
}

#[cfg(test)]
mod tests {
    use std::{fmt, time::Duration};

    use tokio::time;
    use tokio_test::{assert_pending, assert_ready_err, assert_ready_ok, task};
    use tower::retry::RetryLayer;
    use tower_test::{assert_request_eq, mock};

    use super::*;

    #[tokio::test]
    async fn service_error_retry() {
        crate::trace::test_init();

        time::pause();

        let policy = FixedRetryPolicy::new(
            5,
            Duration::from_secs(1),
            Duration::from_secs(10),
            SvcRetryLogic,
        );

        let (mut svc, mut handle) = mock::spawn_layer(RetryLayer::new(policy));

        assert_ready_ok!(svc.poll_ready());

        let fut = svc.call("hello");
        let mut fut = task::spawn(fut);

        assert_request_eq!(handle, "hello").send_error(Error(true));

        assert_pending!(fut.poll());

        time::advance(Duration::from_secs(2)).await;
        assert_pending!(fut.poll());

        assert_request_eq!(handle, "hello").send_response("world");
        assert_eq!(fut.await.unwrap(), "world");
    }

    #[tokio::test]
    async fn service_error_no_retry() {
        crate::trace::test_init();

        let policy = FixedRetryPolicy::new(
            5,
            Duration::from_secs(1),
            Duration::from_secs(10),
            SvcRetryLogic,
        );

        let (mut svc, mut handle) = mock::spawn_layer(RetryLayer::new(policy));

        assert_ready_ok!(svc.poll_ready());

        let mut fut = task::spawn(svc.call("hello"));
        assert_request_eq!(handle, "hello").send_error(Error(false));
        assert_ready_err!(fut.poll());
    }

    #[tokio::test]
    async fn timeout_error() {
        crate::trace::test_init();

        time::pause();

        let policy = FixedRetryPolicy::new(
            5,
            Duration::from_secs(1),
            Duration::from_secs(10),
            SvcRetryLogic,
        );

        let (mut svc, mut handle) = mock::spawn_layer(RetryLayer::new(policy));

        assert_ready_ok!(svc.poll_ready());

        let mut fut = task::spawn(svc.call("hello"));
        assert_request_eq!(handle, "hello").send_error(Elapsed::new());
        assert_pending!(fut.poll());

        time::advance(Duration::from_secs(2)).await;
        assert_pending!(fut.poll());

        assert_request_eq!(handle, "hello").send_response("world");
        assert_eq!(fut.await.unwrap(), "world");
    }

    #[test]
    fn backoff_grows_to_max() {
        let mut policy = FixedRetryPolicy::new(
            10,
            Duration::from_secs(1),
            Duration::from_secs(10),
            SvcRetryLogic,
        );
        assert_eq!(Duration::from_secs(1), policy.backoff());

        policy.advance();
        assert_eq!(Duration::from_secs(1), policy.backoff());

        policy.advance();
        assert_eq!(Duration::from_secs(2), policy.backoff());

        policy.advance();
        assert_eq!(Duration::from_secs(3), policy.backoff());

        policy.advance();
        assert_eq!(Duration::from_secs(5), policy.backoff());

        policy.advance();
        assert_eq!(Duration::from_secs(8), policy.backoff());

        policy.advance();
        assert_eq!(Duration::from_secs(10), policy.backoff());

        policy.advance();
        assert_eq!(Duration::from_secs(10), policy.backoff());
    }

    #[derive(Debug, Clone)]
    struct SvcRetryLogic;

    impl RetryLogic for SvcRetryLogic {
        type Error = Error;
        type Response = &'static str;

        fn is_retriable_error(&self, error: &Self::Error) -> bool {
            error.0
        }
    }

    #[derive(Debug)]
    struct Error(bool);

    impl fmt::Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "error")
        }
    }

    impl std::error::Error for Error {}
}
