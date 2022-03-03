//! A watcher based on the k8s API.

mod builder;
mod chunk;
mod multi_response_decoder;
mod response;
mod watcher;

use builder::WatchRequestBuilder;
use futures::{Stream, StreamExt};
use http::StatusCode;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::WatchEvent;
use k8s_openapi::WatchOptional;
use snafu::ResultExt;

use super::client::Client;

/// A simple watcher atop of the Kubernetes API `Client`
pub struct Watcher<B>
where
    B: 'static,
{
    client: Client,
    request_builder: B,
}

impl<B> Watcher<B>
where
    B: 'static,
{
    /// Create a new Watcher
    pub const fn new(client: Client, request_builder: B) -> Self {
        Self {
            client,
            request_builder,
        }
    }
}

impl<B> Watcher<B>
where
    B: 'static + WatchRequestBuilder,
    <B as WatchRequestBuilder>::Object: Send + Unpin,
{
    async fn invoke(
        &mut self,
        watch_optional: WatchOptional<'_>,
    ) -> Result<
        impl Stream<
                Item = Result<
                    WatchEvent<<B as WatchRequestBuilder>::Object>,
                    watcher::stream::Error<stream::Error>,
                >,
            > + 'static,
        watcher::invocation::Error<invocation::Error>,
    > {
        // Prepare request
        let req = self
            .request_builder
            .build(watch_optional)
            .context(invocation::RequestPreparation)?;

        // Send request, get response
        let resp = match self.client.send(req).await {
            Ok(resp) => resp,
            Err(source @ framework::http::HttpError::CallRequest { .. }) => {
                return Err(watcher::invocation::Error::recoverable(
                    invocation::Error::Request { source },
                ));
            }
            Err(source) => {
                return Err(watcher::invocation::Error::other(
                    invocation::Error::Request { source },
                ));
            }
        };

        // Handle response status code
        let status = resp.status();
        if status != StatusCode::OK {
            let source = invocation::Error::BadStatus { status };
            let err = if status == StatusCode::GONE {
                watcher::invocation::Error::desync(source)
            } else {
                watcher::invocation::Error::other(source)
            };

            return Err(err);
        }

        // Stream response body
        let body = resp.into_body();
        Ok(chunk::body(body).map(|item| match item {
            Ok(WatchEvent::ErrorStatus(status)) if status.code == Some(410) => {
                // HTTP 410 GONE
                Err(watcher::stream::Error::desync(stream::Error::Desync))
            }
            Ok(val) => Ok(val),
            Err(err) => Err(watcher::stream::Error::recoverable(
                stream::Error::KubernetesStream { source: err },
            )),
        }))
    }
}

pub mod invocation {
    use super::*;
    use snafu::Snafu;

    /// Errors that can occur while watching
    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub))]
    pub enum Error {
        /// Returned when the call-specific
        #[snafu(display("failed to prepare an HTTP request"))]
        RequestPreparation {
            /// The underlying error
            source: k8s_openapi::RequestError,
        },

        /// Returned when the HTTP client fails to perform an HTTP request
        #[snafu(display("error during the HTTP request"))]
        Request {
            /// The error that API client returned
            source: framework::http::HttpError,
        },

        /// Returned when the HTTP response has a bad status
        #[snafu(display("HTTP response has a bad status: {}", status))]
        BadStatus {
            /// The status from the HTTP response
            status: StatusCode,
        },
    }

    impl From<Error> for watcher::invocation::Error<Error> {
        fn from(err: Error) -> Self {
            watcher::invocation::Error::other(err)
        }
    }
}

pub mod stream {
    //! Stream error
    use super::*;
    use snafu::Snafu;

    /// Errors that can occur while streaming the watch response.
    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub))]
    pub enum Error {
        /// Returned when the stream-specific error occurs.
        #[snafu(display("kubernetes stream error"))]
        KubernetesStream {
            source: super::chunk::Error<hyper::Error>,
        },
        /// Returned when desync watch response is detected.
        #[snafu(display("desync"))]
        Desync,
    }

    impl From<Error> for watcher::invocation::Error<Error> {
        fn from(err: Error) -> Self {
            watcher::invocation::Error::other(err)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_matches {
        ($expression:expr, $($pattern:tt)+) => {
            match $expression {
                $($pattern)+ => (),
                ref e => panic!("assertion failed: `{:?}` does not match `{}`", e, stringify!($($pattern)+))
            }
        };
    }
}
