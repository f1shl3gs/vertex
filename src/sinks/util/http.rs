use crate::http::HttpError;
use crate::sinks::util::retries::{RetryAction, RetryLogic};
use crate::sinks::util::sink;
use bytes::Bytes;
use http::StatusCode;
use std::fmt;

#[derive(Clone, Debug, Default)]
pub struct HttpRetryLogic;

impl RetryLogic for HttpRetryLogic {
    type Error = HttpError;
    type Response = hyper::Response<Bytes>;

    fn is_retriable_error(&self, error: &Self::Error) -> bool {
        true
    }

    fn should_retry_resp(&self, resp: &Self::Response) -> RetryAction {
        let status = resp.status();

        match status {
            StatusCode::TOO_MANY_REQUESTS => RetryAction::Retry("too many requests".into()),
            StatusCode::NOT_IMPLEMENTED => RetryAction::Retry("endpoint not implemented".into()),
            _ if status.is_server_error() => RetryAction::Retry(
                format!("{}: {}", status, String::from_utf8_lossy(resp.body())).into(),
            ),
            _ if status.is_success() => RetryAction::Successful,
            _ => RetryAction::DontRetry(format!("response status: {}", status).into()),
        }
    }
}

impl<T: fmt::Debug> sink::Response for http::Response<T> {
    fn is_successful(&self) -> bool {
        self.status().is_success()
    }

    fn is_transient(&self) -> bool {
        self.status().is_server_error()
    }
}
