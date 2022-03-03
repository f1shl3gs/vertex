//! Work with HTTP bodies as streams of Kubernetes resources

use async_stream::try_stream;
use bytes::Buf;
use futures::{pin_mut, Stream};
use hyper::body::HttpBody;
use snafu::{ResultExt, Snafu};

use super::multi_response_decoder::MultiResponseDecoder;
use super::response::{self, Response};

/// Errors that can occur in the stream.
#[derive(Debug, Snafu)]
pub enum Error<R>
where
    R: std::error::Error + 'static,
{
    /// An error occurred while reading the response body
    #[snafu(display("reading the data chunk failed"))]
    Reading {
        /// The error we got while reading
        source: R,
    },

    /// An error occurred while parsing the response body
    #[snafu(display("data parsing failed"))]
    Parsing {
        /// Response parsing error
        source: response::Error,
    },

    /// An incomplete response remains in the buffer, but we don't expect
    /// any more data.
    #[snafu(display("unparsed data remaining upon completion"))]
    UnparsedDataUponCompletion {
        /// The unparsed data.
        data: Vec<u8>,
    },
}

/// Convert the HTTP response `Body` to a stream of parsed Kubernetes `Response`s.
pub fn body<B, T>(body: B) -> impl Stream<Item = Result<T, Error<<B as HttpBody>::Error>>>
where
    T: Response + Unpin + 'static,
    B: HttpBody,
    <B as HttpBody>::Error: std::error::Error + Unpin + 'static,
{
    try_stream! {
        let mut decoder: MultiResponseDecoder<T> = MultiResponseDecoder::new();

        debug!(message = "Streaming the HTTP body");

        pin_mut!(body);
        while let Some(buf) = body.data().await {
            let buf = buf.context(Reading)?;
            let chunk = buf.chunk();
            let responses = decoder.process_next_chunk(chunk.as_ref());
            for resp in responses {
                let resp = resp.context(Parsing)?;
                yield resp;
            }
        }

        decoder.finish().map_err(|data| Error::UnparsedDataUponCompletion { data })?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::StreamExt;
    use k8s_openapi::api::core::v1::Pod;
    use k8s_openapi::apimachinery::pkg::apis::meta::v1::WatchEvent;

    fn hyper_body_from_chunks(
        chunks: Vec<Result<&'static str, std::io::Error>>,
    ) -> hyper::body::Body {
        let in_stream = futures::stream::iter(chunks);
        hyper::body::Body::wrap_stream(in_stream)
    }

    #[tokio::test]
    async fn test_body() {
        let data = r#"{
                "type": "ADDED",
                "object": {
                    "kind": "Pod",
                    "apiVersion": "v1",
                    "metadata": {
                        "uid": "uid0"
                    }
                }
            }"#;
        let chunks: Vec<Result<_, std::io::Error>> = vec![Ok(data)];
        let sample_body = hyper_body_from_chunks(chunks);

        let out_stream = body::<_, WatchEvent<Pod>>(sample_body);
        pin_mut!(out_stream);

        assert!(out_stream.next().await.unwrap().is_ok());
        assert!(out_stream.next().await.is_none());
    }

    #[tokio::test]
    async fn body_passes_reading_error() {
        let err = std::io::Error::new(std::io::ErrorKind::Other, "test");
        let chunks: Vec<Result<_, std::io::Error>> = vec![Err(err)];
        let sample_body = hyper_body_from_chunks(chunks);

        let out_stream = body::<_, WatchEvent<Pod>>(sample_body);
        pin_mut!(out_stream);

        let err = out_stream.next().await.unwrap().unwrap_err();
        assert!(matches!(
            err,
            Error::Reading {
                source: hyper::Error { .. }
            }
        ));

        assert!(out_stream.next().await.is_none());
    }

    #[tokio::test]
    async fn body_passes_parsing_error() {
        let chunks: Vec<Result<_, std::io::Error>> = vec![Ok("qwerty")];
        let sample_body = hyper_body_from_chunks(chunks);

        let out_stream = body::<_, WatchEvent<Pod>>(sample_body);
        pin_mut!(out_stream);

        let err = out_stream.next().await.unwrap().unwrap_err();
        assert!(matches!(
            err,
            Error::Parsing {
                source: response::Error::Json(_)
            }
        ));

        assert!(out_stream.next().await.is_none());
    }

    #[tokio::test]
    async fn body_uses_finish() {
        let chunks: Vec<Result<_, std::io::Error>> = vec![Ok("{")];
        let sample_body = hyper_body_from_chunks(chunks);

        let out_stream = body::<_, WatchEvent<Pod>>(sample_body);
        pin_mut!(out_stream);

        let err = out_stream.next().await.unwrap().unwrap_err();
        assert!(matches!(
            err,
            Error::UnparsedDataUponCompletion {
                data
            } if data == vec![b'[']
        ));

        assert!(out_stream.next().await.is_none());
    }
}
