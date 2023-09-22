use bytes::Buf;
use chrono::{DateTime, NaiveDateTime, Utc};
use framework::http::{HttpClient, HttpError};
use framework::sink::util::sink::Response;
use http::StatusCode;
use hyper::Body;
use serde::Deserialize;

mod v2;
mod v3;

/// Gauge represents a single gauge value.
#[derive(Deserialize)]
pub struct Gauge {
    pub name: String,
    #[serde(rename = "counter")]
    pub value: u64,
}

/// Counter represents a single counter value.
#[derive(Deserialize)]
pub struct Counter {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(alias = "$value")]
    pub counter: u64,
}

/// Server represents BIND server statistics
#[derive(Deserialize)]
pub struct Server {
    pub boot_time: DateTime<Utc>,
    pub config_time: DateTime<Utc>,
    pub incoming_queries: Vec<Counter>,
    pub incoming_requests: Vec<Counter>,
    pub name_server_stats: Vec<Counter>,
    pub zone_statistics: Vec<Counter>,
    pub server_rcodes: Vec<Counter>,
}

impl Default for Server {
    fn default() -> Self {
        let zero = DateTime::from_naive_utc_and_offset(
            NaiveDateTime::from_timestamp_millis(0).expect("zero datetime"),
            Utc,
        );

        Self {
            boot_time: zero,
            config_time: zero,
            incoming_queries: vec![],
            incoming_requests: vec![],
            name_server_stats: vec![],
            zone_statistics: vec![],
            server_rcodes: vec![],
        }
    }
}

/// View represents statistics for a single BIND view.
#[derive(Deserialize)]
pub struct View {
    pub name: String,
    pub cache: Vec<Gauge>,
    pub resolver_stats: Vec<Counter>,
    pub resolver_queries: Vec<Counter>,
}

/// ZoneCounter represents a single zone counter value.
#[derive(Deserialize)]
pub struct ZoneCounter {
    pub name: String,
    pub serial: String,
}

/// ZoneView represents statistics for a single BIND zone view.
#[derive(Default, Deserialize)]
pub struct ZoneView {
    pub name: String,
    pub zone_data: Vec<ZoneCounter>,
}

/// ThreadModel contains task and worker information
#[derive(Default, Deserialize)]
pub struct ThreadModel {
    #[serde(rename = "type")]
    pub typ: String,
    #[serde(rename = "worker-threads")]
    pub worker_threads: u64,
    #[serde(rename = "default-quantum")]
    pub default_quantum: u64,
    #[serde(rename = "tasks-running")]
    pub tasks_running: u64,
}

/// TaskManager contains information about all running tasks.
#[derive(Default, Deserialize)]
pub struct TaskManager {
    #[serde(rename = "thread-model")]
    pub thread_model: ThreadModel,
}

/// Statistics is a generic representation of BIND statistics.
#[derive(Default)]
pub struct Statistics {
    pub server: Server,
    pub views: Vec<View>,
    pub zone_views: Vec<ZoneView>,
    pub task_manager: TaskManager,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("http request, {0}")]
    Request(#[from] HttpError),
    #[error("build http request failed, {0}")]
    Http(#[from] http::Error),
    #[error("unexpected status code {0}")]
    UnexpectedStatus(StatusCode),
    #[error("read response body failed, {0}")]
    ReadBody(#[from] hyper::Error),
    #[error("decode response failed, {0}")]
    Decode(#[from] quick_xml::de::DeError),
}

enum Version {
    V2,
    V3,
}

pub struct Client {
    endpoint: String,
    http_client: HttpClient,
}

impl Client {
    pub const fn new(endpoint: String, http_client: HttpClient) -> Self {
        Self {
            endpoint,
            http_client,
        }
    }

    pub async fn stats(&self) -> Result<Statistics, Error> {
        match self.probe().await? {
            Version::V2 => self.v2().await,
            Version::V3 => self.v3().await,
        }
    }

    async fn fetch<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, Error> {
        let url = format!("{}{}", self.endpoint, path);
        let req = http::Request::get(url).body(Body::empty())?;
        let resp = self.http_client.send(req).await?;
        let (header, body) = resp.into_parts();
        if !header.status.is_success() {
            return Err(Error::UnexpectedStatus(header.status));
        }

        let body = hyper::body::to_bytes(body).await?;
        quick_xml::de::from_reader(body.reader()).map_err(Error::Decode)
    }

    async fn probe(&self) -> Result<Version, Error> {
        let url = format!("{}{}", self.endpoint, v3::STATUS_PATH);
        let req = http::Request::get(url).body(Body::empty())?;
        let resp = self.http_client.send(req).await?;

        if resp.is_successful() {
            Ok(Version::V3)
        } else {
            Ok(Version::V2)
        }
    }
}
