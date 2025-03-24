use bytes::Bytes;
use futures::stream::BoxStream;
use futures::{StreamExt, TryStreamExt};
use http::{Method, Request};
use http_body_util::{BodyExt, Full};
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use hyper_util::client::legacy::Client as HttpClient;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::TokioExecutor;
use serde::Deserialize;
use tokio_util::codec::{FramedRead, LinesCodec, LinesCodecError};
use tokio_util::io::StreamReader;
use tracing::trace;

use super::config::{Auth, Config};
use super::resource::Resource;
use super::version::Version;
use super::{ObjectList, config};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Config(config::Error),
    #[error("build http request failed, {0}")]
    BuildRequest(http::Error),
    #[error("read http response failed, {0}")]
    ReadResponse(hyper::Error),
    #[error(transparent)]
    Http(hyper_util::client::legacy::Error),
    #[error("invalid config, {0}")]
    Validation(String),
    #[error("api server error, status: {}, reason: {}, message: {}", .0.status, .0.reason, .0.message)]
    Api(ErrorResponse),
    #[error("deserialize response failed, {0}")]
    Deserialize(serde_json::Error),
    #[error("read watch event failed, {0}")]
    ReadEvents(std::io::Error),
    #[error("chunk line is too large")]
    LinesCodecMaxLineLengthExceeded,
    #[error("refresh token failed, {0}")]
    RefreshToken(std::io::Error),
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Self::Deserialize(err)
    }
}

impl From<hyper::Error> for Error {
    fn from(err: hyper::Error) -> Self {
        Error::ReadResponse(err)
    }
}

impl From<http::Error> for Error {
    fn from(err: http::Error) -> Self {
        Error::BuildRequest(err)
    }
}

/// An error response from the API.
#[derive(Debug, Deserialize)]
pub struct ErrorResponse {
    /// The status
    pub status: String,
    /// A message about the error
    #[serde(default)]
    pub message: String,
    /// The reason for the error
    #[serde(default)]
    pub reason: String,
    /// The error code
    pub code: u16,
}

/// Controls how the resource version parameter is applied for list calls
///
/// Not specifying a `VersionMatch` strategy will give you different semantics
/// depending on what `resource_version`, `limit`, `continue_token` you include
/// with the list request.
///
/// See <https://kubernetes.io/docs/reference/using-api/api-concepts/#semantics-for-get-and-list> for details.
#[derive(Debug, PartialEq)]
pub enum VersionMatch {
    /// Returns data at least as new as the provided resource version.
    ///
    /// The newest available data is preferred, but any data not older than the
    /// provided resource version may be served. This guarantees that the collection's
    /// resource version is not older than the requested resource version, but does
    /// not make any guarantee about the resource version of any of the items in that
    /// collection.
    ///
    /// ### Any Version
    /// A degenerate, but common sub-case of `NotOlderThan` is when used together with
    /// `resource_version` "0".
    ///
    /// It is possible for a "0" resource version request to return data at a much older
    /// resource version than the client has previously observed, particularly in HA
    /// configuration, due to partitions or stale caches. Clients that cannot tolerate
    /// this should not use this semantic.
    NotOlderThan,

    /// Return data at the exact resource version provided.
    ///
    /// If the provided resource version is unavailable, the server responds with HTTP 410
    /// "Gone". For list requests to servers that honor the resource version Match parameter,
    /// this guarantees that the collection's resource version is the same as the resource
    /// version you requested in the query string. That guarantee does not apply to resource
    /// version of any items within that collection.
    ///
    /// Note that `Exact` cannot be used with resource version "0". For the most up-to-date
    /// list; use `Unset`.
    Exact,
}

/// Common query parameters used in list/delete calls on collections
#[derive(Debug, Default)]
pub struct ListParams {
    /// A selector to restrict the list of returned objects by their labels.
    pub label_selector: Option<String>,

    /// A selector to restrict the list of returned objects by their fields.
    pub field_selector: Option<String>,

    /// Timeout for the list/watch call
    ///
    /// This limits the duration of the cal, regardless of any activity or inactivity.
    pub timeout: Option<u32>,

    /// Limit the number of results
    ///
    /// If there are more results, the server will respond with a continue token
    /// which can be used to fetch another page of results.
    ///
    /// See [Kubernetes API docs](https://kubernetes.io/docs/reference/using-api/api-concepts/#retrieving-large-results-sets-in-chunks)
    pub limit: Option<u32>,

    /// Fetch a second page of results.
    ///
    /// After listing results with a limit, a continue token can be used to fetch
    /// another page of results.
    pub continue_token: Option<String>,

    /// Determines how resourceVersion is matched applied to list calls
    pub version_match: Option<VersionMatch>,

    /// An explicit resourceVersion using the given `VersionMatch` strategy
    ///
    /// See <https://kubernetes.io/docs/reference/using-api/api-concepts/#resource-versions> for details.
    pub resource_version: Option<String>,
}

/// Common query parameters used in watch calls on collections
#[derive(Debug, Default)]
pub struct WatchParams {
    /// A selector to restrict returned objects by their labels.
    pub label_selector: Option<String>,

    /// A selector to restrict returned objects by their fields.
    pub field_selector: Option<String>,

    /// Timeout for the watch call.
    ///
    /// This limits the duration of the call, regardless of any activity or inactivity.
    /// If unset for a watch call, we will use 290s. We limit this to 295s due to
    /// [inherent watch limitations](https://github.com/kubernetes/kubernetes/issues/6513).
    pub timeout: Option<u32>,

    /// Enables watch events with type "BOOKMARK"
    ///
    /// Servers that do not implement bookmarks ignore this flag and bookmarks are sent
    /// at the server's discretion. Clients should not assume bookmarks are returned at
    /// any specific interval, nor may they assume the server will send any BOOKMARK
    /// event during a session. If this is not a watch, this field is ignored. If the
    /// feature gate WatchBookmarks is not enabled in apiserver, this field is required.
    pub bookmarks: bool,

    /// Kubernetes 1.27 Streaming Lists `sendInitialEvents=true` may be set together with
    /// `watch=true`. In that case, the watch stream will begin with synthetic events to
    /// produce the current state of objects in the collection. Once all such events have
    /// been sent, a synthetic "Bookmark" event will be sent. The bookmark will report the
    /// ResourceVersion(RV) corresponding to the set of objects, and be marked with
    /// `"k8s.io/initial-events-end": "true"` annotation. Afterwards, the watch stream
    /// will proceed as usual, sending watch events corresponding to changes (subsequent
    /// to the RV) to objects watched.
    ///
    /// When `sendInitialEvents` option is set, we require `resourceVersionMatch` option
    /// to also be set. The semantic of the watch request is as following:
    /// - `resourceVersionMatch` = NotOlderThan is interpreted as "data at least as new as
    ///   the provided `resourceVersion`" and the bookmark event is send when the state is
    ///   synced to a `resourceVersion` at least as fresh as the one provided by the
    ///   ListOptions. If `resourceVersion` is unset, this is interpreted as "consistent
    ///   read" and the bookmark event is send when the state is synced at least to the
    ///   moment when request started being processed.
    /// - `resourceVersionMatch` set to any other value or unset Invalid error is returned.
    pub send_initial_events: bool,
}

#[derive(Deserialize)]
pub struct BookmarkMeta {
    /// The only field we need from a Bookmark event.
    #[serde(rename = "resourceVersion")]
    pub resource_version: String,

    /// Kubernetes 1.27 Streaming Lists
    /// The rest of the fields are optional and may be empty.
    #[serde(default)]
    pub annotations: std::collections::BTreeMap<String, String>,
}

/// Can only be relied upon to have metadata with resource version.
/// Bookmarks contain apiVersion + kind + basically empty metadata
///
/// See https://kubernetes.io/docs/reference/using-api/api-concepts/#watch-bookmarks
#[derive(Deserialize)]
pub struct Bookmark {
    /// Basically empty metadata
    pub metadata: BookmarkMeta,
}

/// A raw event returned from a watch query
///
/// Note that a watch query returns many of these as newline separated JSON
#[derive(Deserialize)]
#[serde(tag = "type", content = "object", rename_all = "UPPERCASE")]
pub enum WatchEvent<K> {
    /// Resource was added
    Added(K),
    /// Resource was modified
    Modified(K),
    /// Resource was deleted
    Deleted(K),
    /// Resource bookmark. `Bookmark` is a slimmed down `K`
    /// From [Watch bookmarks](https://kubernetes.io/docs/reference/using-api/api-concepts/#watch-bookmarks).
    /// NB: This became Beta first in Kubernetes 1.16
    Bookmark(Bookmark),
    /// There was some kind of error
    Error(ErrorResponse),
}

#[derive(Clone)]
pub struct Client {
    http_client: HttpClient<HttpsConnector<HttpConnector>, Full<Bytes>>,
    auth: Auth,
    endpoint: String,
    namespace: Option<String>,
}

impl Client {
    pub fn new(namespace: Option<String>) -> Result<Self, Error> {
        let config = Config::load().map_err(Error::Config)?;

        let builder = HttpsConnectorBuilder::new()
            .with_tls_config(config.tls)
            .https_or_http();
        let mut inner = HttpConnector::new();
        inner.enforce_http(false);
        let connector = builder.enable_http1().wrap_connector(inner);

        let http_client =
            hyper_util::client::legacy::Client::builder(TokioExecutor::new()).build(connector);

        // TOO UGLY
        let endpoint = config
            .cluster_url
            .to_string()
            .strip_suffix("/")
            .unwrap()
            .to_string();

        Ok(Client {
            http_client,
            endpoint,
            auth: config.auth,
            namespace,
        })
    }

    pub fn set_namespace(&mut self, namespace: Option<String>) {
        self.namespace = namespace;
    }

    /// Retrieve version info of the API server, so we can check the compatibility
    pub async fn version(&self) -> Result<Version, Error> {
        let mut req = Request::builder()
            .method(Method::GET)
            .uri(format!("{}/version", self.endpoint))
            .body(Full::<Bytes>::default())?;

        self.auth.apply(&mut req).map_err(Error::RefreshToken)?;

        let resp = self.http_client.request(req).await.map_err(Error::Http)?;
        let (parts, incoming) = resp.into_parts();
        let body = incoming.collect().await?.to_bytes();

        if parts.status.is_success() {
            serde_json::from_slice(&body).map_err(Error::Deserialize)
        } else {
            let err = serde_json::from_slice::<ErrorResponse>(&body)?;
            Err(Error::Api(err))
        }
    }

    /// List a collection of a resource
    pub async fn list<R: Resource>(&self, params: &ListParams) -> Result<ObjectList<R>, Error> {
        // validate params
        if let Some(rv) = &params.resource_version {
            if params.version_match == Some(VersionMatch::Exact) && rv == "0" {
                return Err(Error::Validation(
                    "A non-zero resource_version is required when using an Exact match".into(),
                ));
            }
        } else if params.version_match.is_some() {
            return Err(Error::Validation(
                "A resource_version is required when using an explicit match".into(),
            ));
        }

        let query = {
            let mut builder = form_urlencoded::Serializer::new(String::new());

            if let Some(field_selector) = &params.field_selector {
                builder.append_pair("fieldSelector", field_selector);
            }
            if let Some(label_selector) = &params.label_selector {
                builder.append_pair("labelSelector", label_selector);
            }
            if let Some(limit) = &params.limit {
                builder.append_pair("limit", &limit.to_string());
            }
            if let Some(continue_token) = &params.continue_token {
                builder.append_pair("continue", continue_token);
            } else {
                // When there's a continue token, we don't want to set resourceVersion
                if let Some(resource_version) = &params.resource_version {
                    if resource_version != "0" || params.limit.is_none() {
                        builder.append_pair("resourceVersion", resource_version);

                        match params.version_match {
                            None => {}
                            Some(VersionMatch::NotOlderThan) => {
                                builder.append_pair("resourceVersionMatch", "NotOlderThan");
                            }
                            Some(VersionMatch::Exact) => {
                                builder.append_pair("resourceVersionMatch", "Exact");
                            }
                        }
                    }
                }
            }

            builder.finish()
        };

        let req = Request::builder()
            .method(Method::GET)
            .uri(format!(
                "{}{}?{}",
                self.endpoint,
                R::url_path(self.namespace.as_deref()),
                query
            ))
            .body(Full::<Bytes>::default())?;

        let resp = self.http_client.request(req).await.map_err(Error::Http)?;
        let (parts, incoming) = resp.into_parts();
        let body = incoming.collect().await?.to_bytes();
        if !parts.status.is_success() {
            let err = serde_json::from_slice::<ErrorResponse>(&body).map_err(Error::Deserialize)?;
            return Err(Error::Api(err));
        }

        serde_json::from_slice::<ObjectList<R>>(&body).map_err(Error::Deserialize)
    }

    /// watch returns a stream the produce WatchEvent<R>, and it will stop if
    /// an error occurred or the connection timeout. So, user have to call this
    /// function again to get notified again.
    pub async fn watch<R: Resource>(
        &self,
        params: &WatchParams,
        version: &str,
    ) -> Result<BoxStream<'static, Result<WatchEvent<R>, Error>>, Error> {
        // validate
        if let Some(timeout) = params.timeout {
            if timeout >= 295 {
                return Err(Error::Validation("invalid timeout limit".into()));
            }
        }

        let query = {
            let mut builder = form_urlencoded::Serializer::new(String::new());

            builder.append_pair("resourceVersion", version);
            builder.append_pair("watch", "true");
            // https://github.com/kubernetes/kubernetes/issues/6513
            builder.append_pair(
                "timeoutSeconds",
                params.timeout.unwrap_or(290).to_string().as_str(),
            );

            if let Some(label_selector) = params.label_selector.as_ref() {
                builder.append_pair("labelSelector", label_selector);
            }
            if let Some(field_selector) = params.field_selector.as_ref() {
                builder.append_pair("fieldSelector", field_selector);
            }

            if params.bookmarks {
                builder.append_pair("allowWatchBookmarks", "true");
            }
            if params.send_initial_events {
                builder.append_pair("sendInitialEvents", "true");
                builder.append_pair("resourceVersionMatch", "NotOlderThan");
            }

            builder.finish()
        };
        let uri = format!(
            "{}{}?{}",
            self.endpoint,
            R::url_path(self.namespace.as_deref()),
            query
        );

        trace!(message = "doing http request", uri);

        let mut req = Request::builder()
            .method(Method::GET)
            .uri(uri)
            .body(Full::<Bytes>::default())?;

        self.auth.apply(&mut req).map_err(Error::RefreshToken)?;

        self.request_events(req).await
    }

    async fn request_events<R: Resource>(
        &self,
        req: Request<Full<Bytes>>,
    ) -> Result<BoxStream<'static, Result<WatchEvent<R>, Error>>, Error> {
        let resp = self.http_client.request(req).await.map_err(Error::Http)?;

        let frames = FramedRead::new(
            StreamReader::new(resp.into_body().into_data_stream().map_err(|err| {
                // Unexpected EOF from chunked decoder.
                // Tends to happen when watching for 300+s. This will be ignored
                if err.to_string().contains("unexpected EOF during check") {
                    return std::io::Error::new(std::io::ErrorKind::UnexpectedEof, err);
                }

                std::io::Error::other(err)
            })),
            LinesCodec::new(),
        );

        Ok(Box::pin(frames.filter_map(|result| async {
            match result {
                Ok(line) => {
                    match serde_json::from_str::<WatchEvent<R>>(&line) {
                        Ok(event) => Some(Ok(event)),
                        Err(err) => {
                            // Ignore EOF error that can happen for incomplete line from `decode_eof`.
                            if err.is_eof() {
                                return None;
                            }

                            // Got general error response
                            if let Ok(e_resp) = serde_json::from_str::<ErrorResponse>(&line) {
                                return Some(Err(Error::Api(e_resp)));
                            }
                            // Parsing error
                            Some(Err(Error::Deserialize(err)))
                        }
                    }
                }
                Err(LinesCodecError::Io(err)) => match err.kind() {
                    // Client timeout
                    std::io::ErrorKind::TimedOut => {
                        tracing::warn!("timeout in poll: {}", err); // our client timeout
                        None
                    }
                    // Unexpected EOF from chunked decoder.
                    // Tends to happen after 300+s of watching.
                    std::io::ErrorKind::UnexpectedEof => {
                        tracing::warn!("eof in poll: {}", err);
                        None
                    }
                    _ => Some(Err(Error::ReadEvents(err))),
                },

                // Reached the maximum line length without finding a newline.
                // This should never happen because we're using the default `usize::MAX`.
                Err(LinesCodecError::MaxLineLengthExceeded) => {
                    Some(Err(Error::LinesCodecMaxLineLengthExceeded))
                }
            }
        })))
    }
}
