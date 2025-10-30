//! This extension runs as a Web server that loads the remote observers that are registered against it.
//!
//! It allows users of the collectors to visualize data going through pipelines.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;

use bytes::Bytes;
use configurable::configurable_component;
use framework::Extension;
use framework::config::{ExtensionConfig, ExtensionContext, Resource};
use framework::http::{Auth, Authorizer};
use framework::observe::current_endpoints;
use framework::tls::MaybeTlsListener;
use http::header::CONTENT_TYPE;
use http::{Request, Response, StatusCode};
use http_body_util::Full;
use hyper::body::Incoming;
use metrics::{Attributes, Observation, Reporter, global_registry};
use parking_lot::RwLock;
use serde::Serialize;

static CURRENT_CONFIG: RwLock<String> = RwLock::new(String::new());

pub fn update_config(config: &framework::config::Config) {
    let content = serde_yaml::to_string(config).unwrap();
    *CURRENT_CONFIG.write() = content;
}

fn default_listen() -> SocketAddr {
    SocketAddr::from(([0, 0, 0, 0], 11000))
}

#[configurable_component(extension, name = "remote_tap")]
struct Config {
    /// The address in which the web server will be listening to.
    #[serde(default = "default_listen")]
    listen: SocketAddr,

    auth: Option<Auth>,
}

#[async_trait::async_trait]
#[typetag::serde(name = "remote_tap")]
impl ExtensionConfig for Config {
    async fn build(&self, cx: ExtensionContext) -> crate::Result<Extension> {
        let shutdown = cx.shutdown;
        let listener = MaybeTlsListener::bind(&self.listen, None).await?;
        let svc = Service {
            auth: self.auth.as_ref().map(|auth| Arc::new(auth.authorizer())),
        };

        Ok(Box::pin(async move {
            if let Err(err) = framework::http::serve(listener, svc)
                .with_graceful_shutdown(shutdown)
                .await
            {
                warn!(message = "http server exited with error", ?err);
            }

            Ok(())
        }))
    }

    fn resources(&self) -> Vec<Resource> {
        vec![Resource::tcp(self.listen)]
    }
}

const INDEX_PAGE: &[u8] = include_bytes!("static/index.html");

#[derive(Clone)]
struct Service {
    auth: Option<Arc<Authorizer>>,
}

impl hyper::service::Service<Request<Incoming>> for Service {
    type Response = Response<Full<Bytes>>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<Incoming>) -> Self::Future {
        if let Some(authorizer) = &self.auth
            && !authorizer.authorized(&req)
        {
            let resp = Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .body(Full::<Bytes>::default())
                .unwrap();

            return Box::pin(async move { Ok(resp) });
        }

        Box::pin(async move {
            let resp = match req.uri().path() {
                "/" => Response::builder()
                    .status(StatusCode::OK)
                    .header(CONTENT_TYPE, "text/html; charset=utf-8")
                    .body(Full::<Bytes>::new(Bytes::from_static(INDEX_PAGE)))
                    .unwrap(),
                "/stats" => {
                    let stats = Stats::snapshot();
                    let body = serde_json::to_vec(&stats.metrics).unwrap();

                    Response::builder()
                        .status(StatusCode::OK)
                        .header(CONTENT_TYPE, "application/json")
                        .body(Full::new(Bytes::from(body)))
                        .unwrap()
                }
                "/config" => {
                    let text = CURRENT_CONFIG.read().to_string();

                    Response::builder()
                        .header(CONTENT_TYPE, "text/yaml; charset=utf-8")
                        .status(StatusCode::OK)
                        .body(Full::<Bytes>::new(Bytes::from(text)))
                        .unwrap()
                }
                "/observers" => {
                    let endpoints = current_endpoints();
                    let body = serde_json::to_vec(&endpoints).unwrap();

                    Response::builder()
                        .header(CONTENT_TYPE, "application/json")
                        .status(StatusCode::OK)
                        .body(Full::new(Bytes::from(body)))
                        .unwrap()
                }
                _ => Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Full::<Bytes>::default())
                    .unwrap(),
            };

            Ok(resp)
        })
    }
}

#[derive(Debug, Serialize)]
struct Point {
    attrs: HashMap<String, String>,
    value: f64,
}

#[derive(Debug, Serialize)]
struct Metric {
    name: &'static str,
    description: &'static str,
    points: Vec<Point>,
}

#[derive(Debug)]
struct Stats {
    metrics: Vec<Metric>,
    state: Option<Metric>,
}

impl Reporter for Stats {
    fn start_metric(&mut self, name: &'static str, description: &'static str) {
        self.state = Some(Metric {
            name,
            description,
            points: Vec::new(),
        })
    }

    fn report(&mut self, attrs: &Attributes, observation: Observation) {
        let metric = if let Some(metric) = &mut self.state {
            metric
        } else {
            return;
        };

        let value = match observation {
            Observation::Counter(c) => c as f64,
            Observation::Gauge(g) => g,
            Observation::Histogram(_h) => {
                return;
            }
        };

        let attrs = attrs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<HashMap<String, String>>();

        metric.points.push(Point { attrs, value });
    }

    fn finish_metric(&mut self) {
        if let Some(state) = self.state.take() {
            self.metrics.push(state);
        }
    }
}

impl Stats {
    fn snapshot() -> Self {
        let mut stats = Stats {
            metrics: Vec::new(),
            state: None,
        };

        global_registry().report(&mut stats);

        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>();
    }
}
