pub mod containers;
pub mod system;

use std::collections::HashMap;
use std::fmt::Display;
use std::path::PathBuf;

use bytes::{Buf, Bytes, BytesMut};
use futures::{Stream, TryStreamExt};
use http::{Method, Request};
use http_body_util::{BodyExt, BodyStream, Full};
use hyper_unix::UnixConnector;
use hyper_util::rt::TokioExecutor;
use percent_encoding::{NON_ALPHANUMERIC, percent_encode};
use tokio_util::codec::{Decoder, FramedRead};
use tokio_util::io::StreamReader;

#[derive(Debug)]
pub enum Error {
    UnexpectedStatusCode {
        code: http::StatusCode,
        body: String,
    },

    Http(http::Error),

    Hyper(hyper::Error),

    Client(hyper_util::client::legacy::Error),

    Deserialize(serde_json::Error),

    Io(std::io::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::UnexpectedStatusCode { code, body } => {
                write!(f, "unexpected status code {code}, body: {body}")
            }
            Error::Http(err) => err.fmt(f),
            Error::Hyper(err) => err.fmt(f),
            Error::Client(err) => err.fmt(f),
            Error::Deserialize(err) => err.fmt(f),
            Error::Io(err) => err.fmt(f),
        }
    }
}

impl From<http::Error> for Error {
    fn from(err: http::Error) -> Self {
        Error::Http(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

fn encode_filters<T: serde::Serialize>(map: &HashMap<T, Vec<T>>) -> String {
    if map.is_empty() {
        return String::new();
    }

    let filters = serde_json::to_string(map).unwrap();

    percent_encode(filters.as_bytes(), NON_ALPHANUMERIC).to_string()
}

#[derive(Clone)]
pub struct Client {
    http: hyper_util::client::legacy::Client<UnixConnector, Full<Bytes>>,
}

impl Default for Client {
    fn default() -> Self {
        Self::new(PathBuf::from("/var/run/docker.sock"))
    }
}

impl Client {
    pub fn new(path: PathBuf) -> Self {
        let connector = UnixConnector::new(path);
        let http =
            hyper_util::client::legacy::Client::builder(TokioExecutor::new()).build(connector);

        Self { http }
    }

    async fn fetch<T: serde::de::DeserializeOwned>(&self, uri: String) -> Result<T, Error> {
        let req = Request::builder()
            .method(Method::GET)
            .uri(uri)
            .body(Full::<Bytes>::default())?;

        let resp = self.http.request(req).await.map_err(Error::Client)?;
        let (parts, incoming) = resp.into_parts();
        if !parts.status.is_success() {
            let data = incoming.collect().await.map_err(Error::Hyper)?.to_bytes();

            return Err(Error::UnexpectedStatusCode {
                code: parts.status,
                body: String::from_utf8_lossy(&data).into_owned(),
            });
        }

        let data = incoming.collect().await.map_err(Error::Hyper)?.to_bytes();

        serde_json::from_slice(&data).map_err(Error::Deserialize)
    }

    async fn stream(&self, uri: String) -> Result<impl Stream<Item = Result<Bytes, Error>>, Error> {
        let req = Request::builder()
            .method(Method::GET)
            .uri(uri)
            .body(Full::<Bytes>::default())?;

        let resp = self.http.request(req).await.map_err(Error::Client)?;
        let (parts, incoming) = resp.into_parts();
        if !parts.status.is_success() {
            let data = incoming.collect().await.map_err(Error::Hyper)?.to_bytes();

            return Err(Error::UnexpectedStatusCode {
                code: parts.status,
                body: String::from_utf8_lossy(&data).into_owned(),
            });
        }

        let frames = FramedRead::new(
            StreamReader::new(
                Box::pin(
                    BodyStream::new(incoming)
                        .try_filter_map(|frame| async { Ok(frame.into_data().ok()) }),
                )
                .map_err(|err| {
                    // Client timeout. This will be ignored.
                    if err.is_timeout() {
                        return std::io::Error::new(std::io::ErrorKind::TimedOut, err);
                    }

                    // Unexpected EOF from chunked decoder.
                    // Tends to happen when watching for 300+s. This will be ignored.
                    if err.to_string().contains("unexpected EOF during chunk") {
                        return std::io::Error::new(std::io::ErrorKind::UnexpectedEof, err);
                    }

                    std::io::Error::other(err)
                }),
            ),
            LinesCodec,
        );

        Ok(frames)
    }
}

struct LinesCodec;

impl Decoder for LinesCodec {
    type Item = Bytes;
    type Error = Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if buf.is_empty() {
            return Ok(None);
        }

        match buf.iter().position(|b| *b == b'\n') {
            Some(pos) => {
                let data = buf.split_to(pos);
                buf.advance(1);

                Ok(Some(data.freeze()))
            }
            None => {
                // No newline delimited
                Ok(None)
            }
        }
    }
}
