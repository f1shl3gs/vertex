use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};

use http_body_util::{BodyExt, BodyStream, Full};
use hyper::body::{Bytes, Incoming};
use hyper::rt::ReadBufCursor;
use hyper::{Method, Request, StatusCode, Uri};
use hyper_util::client::legacy::connect::{Connected, Connection};
use hyper_util::rt::{TokioExecutor, TokioIo};
use percent_encoding::NON_ALPHANUMERIC;
use pin_project_lite::pin_project;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWrite;
use tokio_util::bytes::BytesMut;
use tower::Service;
use tracing::{info, warn};

pin_project! {
    pub struct UnixStream {
        #[pin]
        inner: tokio::net::UnixStream,
    }
}

impl hyper::rt::Read for UnixStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: ReadBufCursor<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        let mut t = TokioIo::new(self.project().inner);
        Pin::new(&mut t).poll_read(cx, buf)
    }
}

impl hyper::rt::Write for UnixStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        self.project().inner.poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        self.project().inner.poll_flush(cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        self.project().inner.poll_shutdown(cx)
    }
}

impl Connection for UnixStream {
    fn connected(&self) -> Connected {
        Connected::new()
    }
}

#[derive(Clone)]
pub struct UnixConnector(PathBuf);

impl Service<Uri> for UnixConnector {
    type Response = UnixStream;
    type Error = std::io::Error;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _req: Uri) -> Self::Future {
        let path = self.0.clone();

        Box::pin(async move {
            let inner = tokio::net::UnixStream::connect(path).await?;

            Ok(UnixStream { inner })
        })
    }
}

#[derive(Deserialize)]
struct ErrResp {
    message: String,
}

#[derive(Debug)]
pub enum Error {
    ReadResponse(hyper::Error),
    Http(hyper::http::Error),
    Request(hyper_util::client::legacy::Error),
    Deserialize(serde_json::Error),
    Api(StatusCode, String),

    AlreadyRunning,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ReadResponse(err) => write!(f, "failed to read response: {}", err),
            Error::Http(err) => write!(f, "{}", err),
            Error::Request(err) => write!(f, "request failed: {}", err),
            Error::Deserialize(err) => err.fmt(f),
            Error::Api(code, err) => {
                write!(f, "docker engine error, code: {} message: {}", code, err)
            }
            Error::AlreadyRunning => f.write_str("container already running"),
        }
    }
}

impl From<hyper::Error> for Error {
    fn from(err: hyper::Error) -> Self {
        Error::ReadResponse(err)
    }
}

impl From<hyper::http::Error> for Error {
    fn from(err: hyper::http::Error) -> Self {
        Error::Http(err)
    }
}

impl From<hyper_util::client::legacy::Error> for Error {
    fn from(err: hyper_util::client::legacy::Error) -> Self {
        Error::Request(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Deserialize(err)
    }
}

#[derive(Debug, Serialize)]
pub struct PortBinding {
    #[serde(rename = "HostIp")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_ip: Option<String>,

    #[serde(rename = "HostPort")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_port: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HostConfig {
    #[serde(rename = "ExtraHosts")]
    pub extra_hosts: Vec<String>,
    #[serde(rename = "Binds")]
    pub binds: Vec<String>,
    #[serde(rename = "PortBindings")]
    pub port_bindings: HashMap<String, Vec<PortBinding>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreateOptions {
    pub image: String,
    pub env: Vec<String>,
    pub cmd: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exposed_ports: Option<HashMap<String, HashMap<(), ()>>>,
    pub host_config: HostConfig,
}

#[derive(Clone)]
pub struct Client {
    client: hyper_util::client::legacy::Client<UnixConnector, Full<Bytes>>,
}

impl Default for Client {
    fn default() -> Self {
        Self::new("/var/run/docker.sock")
    }
}

impl Client {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            client: hyper_util::client::legacy::Client::builder(TokioExecutor::new())
                .build(UnixConnector(path.into())),
        }
    }

    pub async fn pull(&self, image: &str, tag: &str) -> Result<(), Error> {
        #[allow(dead_code)]
        #[derive(Deserialize)]
        struct Image {
            #[serde(rename = "Id")]
            id: String,
        }

        let mut filters = HashMap::new();
        filters.insert("reference".to_string(), vec![format!("{}:{}", image, tag)]);

        let fv = serde_json::to_string(&filters).unwrap();
        let filters = percent_encoding::utf8_percent_encode(&fv, NON_ALPHANUMERIC);
        let uri = format!("http://localhost/images/json?filters={}", filters);
        let req = Request::builder()
            .method(Method::GET)
            .uri(&uri)
            .header("Accept", "application/json")
            .body(Full::default())
            .unwrap();

        let resp = self.client.request(req).await?;
        let (parts, incoming) = resp.into_parts();

        let data = incoming.collect().await?.to_bytes();

        if !parts.status.is_success() {
            let resp = serde_json::from_slice::<ErrResp>(&data)?;
            return Err(Error::Api(parts.status, resp.message));
        }

        let images = serde_json::from_slice::<Vec<Image>>(&data)?;
        if !images.is_empty() {
            // found
            return Ok(());
        }

        info!(message = "image not found locally", image, tag);

        let uri = format!(
            "http://localhost/images/create?fromImage={}&tag={}",
            image, tag
        );
        let req = Request::builder()
            .method(Method::POST)
            .uri(uri)
            .header(hyper::header::CONTENT_TYPE, "application/json")
            .body(Full::default())
            .unwrap();

        let resp = self.client.request(req).await?;

        let (parts, incoming) = resp.into_parts();
        let data = incoming.collect().await?.to_bytes();

        if parts.status.is_success() {
            return Ok(());
        }

        let resp = serde_json::from_slice::<ErrResp>(&data)?;

        Err(Error::Api(parts.status, resp.message))
    }

    pub async fn create(&self, options: CreateOptions) -> Result<String, Error> {
        #[derive(Debug, Deserialize)]
        struct CreateResp {
            #[serde(rename = "Id")]
            id: String,
            #[serde(rename = "Warnings")]
            warnings: Vec<String>,
        }

        let uri = build_uri::<()>("/containers/create", None);
        let data = serde_json::to_vec(&options).unwrap();

        let req = hyper::Request::builder()
            .method(Method::POST)
            .uri(uri)
            .header(hyper::header::CONTENT_TYPE, "application/json")
            .body(Full::from(Bytes::from(data)))?;
        let resp = self.client.request(req).await?;
        let (parts, incoming) = resp.into_parts();

        let data = incoming.collect().await?.to_bytes();

        if parts.status != StatusCode::CREATED {
            let resp = serde_json::from_slice::<ErrResp>(&data)?;
            return Err(Error::Api(parts.status, resp.message));
        }

        let resp = serde_json::from_slice::<CreateResp>(&data)?;

        for warning in resp.warnings {
            warn!(warning);
        }

        Ok(resp.id)
    }

    pub async fn start(&self, id: &str) -> Result<(), Error> {
        let req = Request::builder()
            .method(Method::POST)
            .uri(format!("http://localhost/containers/{}/start", id))
            .body(Full::default())
            .unwrap();

        let res = self.client.request(req).await?;
        let (parts, incoming) = res.into_parts();

        if parts.status == StatusCode::NO_CONTENT {
            Ok(())
        } else if parts.status == StatusCode::NOT_MODIFIED {
            Err(Error::AlreadyRunning)
        } else {
            let data = incoming.collect().await?.to_bytes();

            let resp = serde_json::from_slice::<ErrResp>(&data)?;
            return Err(Error::Api(parts.status, resp.message));
        }
    }

    pub async fn stop(&self, id: &str) -> Result<(), Error> {
        let req = hyper::Request::builder()
            .method(Method::POST)
            .uri(format!("http://localhost/containers/{}/stop", id))
            .body(Full::default())
            .unwrap();

        let resp = self.client.request(req).await?;
        let (parts, incoming) = resp.into_parts();
        if parts.status == StatusCode::NO_CONTENT {
            return Ok(());
        }

        let data = incoming.collect().await?.to_bytes();
        let resp = serde_json::from_slice::<ErrResp>(&data)?;

        Err(Error::Api(parts.status, resp.message))
    }

    pub async fn remove(&self, id: &str) -> Result<(), Error> {
        let req = Request::builder()
            .method(Method::DELETE)
            .uri(format!("http://localhost/containers/{}", id))
            .body(Full::default())
            .unwrap();

        let resp = self.client.request(req).await?;
        let (parts, incoming) = resp.into_parts();
        if parts.status.is_success() {
            return Ok(());
        }

        let data = incoming.collect().await?.to_bytes();
        let resp = serde_json::from_slice::<ErrResp>(&data)?;

        Err(Error::Api(parts.status, resp.message))
    }

    pub async fn tail_logs(
        &self,
        id: &str,
        stdout: bool,
        stderr: bool,
    ) -> Result<BodyStream<Incoming>, Error> {
        let req = Request::builder()
            .method(Method::GET)
            .uri(format!("http://localhost/containers/{id}/logs?stdout={stdout}&stderr={stderr}&follow=true&tail=all"))
            .body(Full::default())?;

        let resp = self.client.request(req).await?;
        let (parts, incoming) = resp.into_parts();
        if parts.status != StatusCode::OK {
            let data = incoming.collect().await?.to_bytes();

            let resp = serde_json::from_slice::<ErrResp>(&data)?;
            return Err(Error::Api(parts.status, resp.message));
        }

        Ok(BodyStream::new(incoming))
    }
}

fn build_uri<Q: Serialize>(path: &str, query: Option<Q>) -> Uri {
    let builder = Uri::builder().scheme("http").authority("localhost");

    let builder = if let Some(query) = query {
        let q = serde_json::to_string(&query).unwrap();
        let query = percent_encoding::utf8_percent_encode(&q, NON_ALPHANUMERIC);
        builder.path_and_query(format!("{}?{}", path, query))
    } else {
        builder.path_and_query(path)
    };

    builder.build().unwrap()
}

enum DecoderState {
    WaitingHeader,
    WaitingPayload(u8, usize), // StreamType, Length
}

pub enum LogOutput {
    Stdout(Bytes),
    Stderr(Bytes),
}

pub struct NewlineLogOutputDecoder {
    state: DecoderState,
}

impl Default for NewlineLogOutputDecoder {
    fn default() -> Self {
        NewlineLogOutputDecoder {
            state: DecoderState::WaitingHeader,
        }
    }
}

impl tokio_util::codec::Decoder for NewlineLogOutputDecoder {
    type Item = LogOutput;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        loop {
            match self.state {
                DecoderState::WaitingHeader => {
                    if src.len() < 8 {
                        return Ok(None);
                    }

                    let header = src.split_to(8);
                    let length = u32::from_be_bytes([header[4], header[5], header[6], header[7]]);

                    self.state = DecoderState::WaitingPayload(header[0], length as usize);
                }
                DecoderState::WaitingPayload(typ, length) => {
                    if src.len() < length {
                        return Ok(None);
                    }

                    let mut msg = src.split_to(length).freeze();
                    if let Some(b'\n') = msg.last() {
                        msg.truncate(length - 1);
                    }

                    let output = match typ {
                        1 => LogOutput::Stdout(msg),
                        2 => LogOutput::Stderr(msg),
                        _ => unreachable!(),
                    };

                    self.state = DecoderState::WaitingHeader;

                    return Ok(Some(output));
                }
            }
        }
    }
}
