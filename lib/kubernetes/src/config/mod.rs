#![allow(dead_code)]

mod file;
mod incluster;
mod tls;

use std::fmt::{Debug, Formatter};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use headers::{Authorization, HeaderMapExt};
use http::Request;
use tracing::error;

/// Errors from loading data from a base64 string or a file
#[derive(Debug, thiserror::Error)]
pub enum LoadDataError {
    /// Failed to decode base64 data
    #[error("failed to decode base64 data: {0}")]
    DecodeBase64(#[source] base64::DecodeError),

    /// Failed to read file
    #[error("failed to read file '{1:?}': {0}")]
    ReadFile(#[source] std::io::Error, PathBuf),

    /// No base64 data or file path was provided
    #[error("missing base64 data or file")]
    MissingDataOrFile,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    InCluster(#[from] incluster::Error),

    #[error(transparent)]
    File(#[from] file::Error),
}

struct Inner {
    token: String,
    expire_at: Instant,
}

#[derive(Clone)]
pub struct RefreshableToken {
    path: PathBuf,
    inner: Arc<Mutex<Inner>>,
}

impl Debug for RefreshableToken {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RefreshableToken")
            .field("path", &self.path)
            .field("expire_at", &self.inner.lock().unwrap().expire_at)
            .finish()
    }
}

impl RefreshableToken {
    pub(crate) fn new(path: PathBuf) -> std::io::Result<Self> {
        let token = std::fs::read_to_string(&path)?;

        Ok(RefreshableToken {
            path,
            inner: Arc::new(Mutex::new(Inner {
                token,
                expire_at: Instant::now(),
            })),
        })
    }

    pub fn token(&self) -> std::io::Result<String> {
        let now = Instant::now();

        let mut inner = self.inner.lock().unwrap();

        if now > inner.expire_at {
            // refresh
            let content = std::fs::read_to_string(&self.path)?;
            inner.token = content;
            inner.expire_at = now + Duration::from_secs(60);
        }

        Ok(inner.token.clone())
    }
}

#[derive(Clone, Debug)]
pub enum Auth {
    None,
    Basic { username: String, password: String },
    Bearer { token: String },
    RefreshableToken(RefreshableToken),
}

impl Auth {
    pub fn apply<T>(&self, req: &mut Request<T>) -> std::io::Result<()> {
        match self {
            Auth::None => {}
            Auth::Basic { username, password } => {
                req.headers_mut()
                    .typed_insert(Authorization::basic(username, password));
            }
            Auth::Bearer { token } => {
                req.headers_mut()
                    .typed_insert(Authorization::bearer(token).unwrap());
            }
            Auth::RefreshableToken(refreshable_token) => {
                let token = refreshable_token.token()?;
                req.headers_mut()
                    .typed_insert(Authorization::bearer(&token).unwrap());
            }
        }

        Ok(())
    }
}

/// Configuration object detailing things like cluster URL, default namespace,
/// root certificates, and timeouts.
///
/// # Usage
/// Construct a [`Config`] instance by using one of the many constructors.
///
/// Prefer [`Config::infer`] unless you have particular issues, and avoid
/// manually managing the data in this struct unless you have particular
/// needs. It exists to be consumed by the [`Client`].
///
/// If you are looking to parse the KubeConfig found in a user's home directory
/// see [`KubeConfig`]
#[derive(Debug)]
pub struct Config {
    /// The configured cluster url.
    pub cluster_url: http::Uri,

    /// The configured default namespace.
    pub default_namespace: String,

    /// Stores information to tell the cluster who you are.
    pub auth: Auth,

    /// Optional proxy URL.
    pub proxy_url: Option<http::Uri>,

    pub tls: rustls::ClientConfig,
}

impl Config {
    pub fn load() -> Result<Config, Error> {
        if let Ok(home) = std::env::var("HOME") {
            let path = format!("{}/.kube/config", home);
            if let Ok(config) = file::from_config(path) {
                return Ok(config);
            }
        }

        incluster::incluster_env().map_err(Into::into)
    }
}
