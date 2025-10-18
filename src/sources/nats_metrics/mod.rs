mod accstatz;
mod connz;
mod gatewayz;
mod healthz;
mod jsz;
mod leafz;
mod routez;
mod varz;

use std::collections::BTreeMap;
use std::panic;
use std::time::Duration;

use crate::sources::nats_metrics::varz::object_to_metrics;
use bytes::Bytes;
use configurable::{Configurable, configurable_component};
use event::Metric;
use framework::config::{OutputType, SourceConfig, SourceContext, default_interval};
use framework::http::{HttpClient, HttpError};
use framework::{Pipeline, ShutdownSignal, Source};
use http::{Method, Request, header};
use http_body_util::{BodyExt, Full};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::task::JoinSet;
use value::Value;

#[derive(Clone, Configurable, Debug, Deserialize, Serialize)]
struct Collectors {
    #[serde(default)]
    varz: bool,
    #[serde(default)]
    connz: bool,
    #[serde(default)]
    accstatz: bool,
    #[serde(default)]
    healthz: bool,
    #[serde(default)]
    jsz: jsz::Config,
    #[serde(default)]
    leafz: bool,
    #[serde(default)]
    routez: bool,
    #[serde(default)]
    subsz: bool,
    #[serde(default)]
    gatewayz: bool,
}

impl Default for Collectors {
    fn default() -> Self {
        Collectors {
            varz: true,
            connz: true,
            accstatz: true,
            healthz: true,
            jsz: Default::default(),
            leafz: true,
            routez: true,
            subsz: true,
            gatewayz: true,
        }
    }
}

#[configurable_component(source, name = "nats_metrics")]
struct Config {
    #[configurable(required)]
    endpoints: Vec<String>,

    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,

    #[serde(default)]
    collectors: Collectors,
}

#[async_trait::async_trait]
#[typetag::serde(name = "nats_metrics")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        if self.endpoints.is_empty() {
            return Err("endpoints is required".into());
        }

        let client = HttpClient::new(None, &cx.proxy)?;
        let endpoints = self.endpoints.clone();
        let interval = self.interval;
        let collectors = self.collectors.clone();
        let output = cx.output;
        let shutdown = cx.shutdown;

        Ok(Box::pin(async move {
            let mut tasks = JoinSet::default();

            for endpoint in endpoints {
                tasks.spawn(run(
                    endpoint,
                    interval,
                    client.clone(),
                    collectors.clone(),
                    output.clone(),
                    shutdown.clone(),
                ));
            }

            while tasks.join_next().await.is_some() {}

            Ok(())
        }))
    }

    fn outputs(&self) -> Vec<OutputType> {
        vec![OutputType::metric()]
    }

    fn can_acknowledge(&self) -> bool {
        false
    }
}

async fn run(
    endpoint: String,
    interval: Duration,
    client: HttpClient,
    collectors: Collectors,
    mut output: Pipeline,
    mut shutdown: ShutdownSignal,
) -> Result<(), ()> {
    let mut ticker = tokio::time::interval(interval);

    println!("{collectors:#?}");

    loop {
        tokio::select! {
            _ = ticker.tick() => {},
            _ = &mut shutdown => break,
        }

        match collect(&client, &endpoint, &collectors).await {
            Ok(metrics) => {
                if let Err(_err) = output.send(metrics).await {
                    break;
                }
            }
            Err(err) => {
                warn!(message = "collect metrics failed", %endpoint, %err);
            }
        }
    }

    Ok(())
}

async fn collect(
    client: &HttpClient,
    endpoint: &str,
    collectors: &Collectors,
) -> Result<Vec<Metric>, Error> {
    let mut metrics = vec![];

    let resp = fetch::<BTreeMap<String, Value>>(client, &format!("{endpoint}/varz")).await?;
    let server_name = resp
        .get("server_name")
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_default();
    if collectors.varz {
        object_to_metrics("gnatsd_varz", resp, &mut metrics);
    }

    if collectors.connz {
        match connz::collect(client, endpoint).await {
            Ok(partial) => {
                metrics.extend(partial);
            }
            Err(err) => {
                warn!(message = "connz collect failed", %endpoint, %err);
            }
        }
    }

    if collectors.accstatz {
        match accstatz::collect(client, endpoint).await {
            Ok(partial) => {
                metrics.extend(partial);
            }
            Err(err) => {
                warn!(message = "accstatz collect failed", %endpoint, %err);
            }
        }
    }

    if collectors.healthz {
        match healthz::collect(client, endpoint).await {
            Ok(partial) => {
                metrics.extend(partial);
            }
            Err(err) => {
                warn!(message = "healthz collect failed", %endpoint, %err);
            }
        }
    }

    if collectors.leafz {
        match leafz::collect(client, endpoint).await {
            Ok(partial) => {
                metrics.extend(partial);
            }
            Err(err) => {
                warn!(message = "leafz collect failed", %endpoint, %err);
            }
        }
    }

    if collectors.gatewayz {
        match gatewayz::collect(client, endpoint).await {
            Ok(partial) => {
                metrics.extend(partial);
            }
            Err(err) => {
                warn!(message = "gatewayz collect failed", %endpoint, %err);
            }
        }
    }

    if collectors.jsz.enabled() {
        match jsz::collect(client, endpoint, &collectors.jsz, &server_name).await {
            Ok(partial) => {
                metrics.extend(partial);
            }
            Err(err) => {
                warn!(message = "jsz collect failed", %endpoint, %err);
            }
        }
    }

    if collectors.routez {
        match routez::collect(client, endpoint).await {
            Ok(partial) => metrics.extend(partial),
            Err(err) => {
                warn!(message = "routez collect failed", %endpoint, %err);
            }
        }
    }

    if collectors.subsz {
        let resp = fetch::<BTreeMap<String, Value>>(client, &format!("{endpoint}/subsz")).await?;
        object_to_metrics("gnatsd_subsz", resp, &mut metrics);
    }

    metrics
        .iter_mut()
        .for_each(|m| m.tags_mut().insert("server", endpoint));

    Ok(metrics)
}

#[derive(Debug, Error)]
enum Error {
    #[error(transparent)]
    Http(#[from] HttpError),

    #[error(transparent)]
    Deserialize(serde_json::error::Error),

    #[error("unexpected status code {0}, resp: {0}")]
    Api(http::StatusCode, String),
}

async fn fetch<T: for<'a> Deserialize<'a>>(client: &HttpClient, uri: &str) -> Result<T, Error> {
    let req = Request::builder()
        .method(Method::GET)
        .uri(uri)
        .header(header::ACCEPT, "application/json")
        .body(Full::<Bytes>::default())
        .map_err(|err| Error::Http(err.into()))?;

    let resp = client.send(req).await?;

    let (parts, incoming) = resp.into_parts();
    let body = incoming
        .collect()
        .await
        .map_err(|err| Error::Http(HttpError::ReadIncoming(err)))?
        .to_bytes();

    if !parts.status.is_success() {
        return Err(Error::Api(
            parts.status,
            String::from_utf8_lossy(&body).to_string(),
        ));
    }

    serde_json::from_slice(&body).map_err(Error::Deserialize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>();
    }

    /*
        #[tokio::test]
        async fn mock() {
            use http::{Response, StatusCode};
            use hyper::body::Incoming;
            use hyper::server::conn::http1;
            use hyper::service::service_fn;
            use hyper_util::rt::TokioIo;
            use std::net::SocketAddr;
            use tokio::net::TcpListener;

            let addr: SocketAddr = "127.0.0.1:15000".parse().unwrap();

            let listener = TcpListener::bind(&addr).await.unwrap();
            println!("listening on {}", addr);

            loop {
                let (stream, _) = listener.accept().await.unwrap();

                tokio::spawn(async move {
                    let service = service_fn(|req: Request<Incoming>| async move {
                        let path = req.uri().path();
                        let path = if path == "/jsz" {
                            match req.uri().query() {
                                Some(query) => {
                                    if query.contains("accounts") {
                                        "/jsz_accounts"
                                    } else if query.contains("consumers") {
                                        "/jsz_consumers"
                                    } else {
                                        "/jsz"
                                    }
                                }
                                None => "/jsz",
                            }
                        } else {
                            path
                        };

                        let path = format!("tests/nats/stats/{}.json", path.strip_prefix("/").unwrap());

                        println!("path: {:?} file: {}", req.uri(), path);

                        match std::fs::read(&path) {
                            Ok(data) => Response::builder()
                                .status(StatusCode::OK)
                                .header("Content-Type", "application/json")
                                .body(Full::<Bytes>::new(data.into())),
                            Err(err) => {
                                println!("Failed to read file {path:?}, {err}");
                                Response::builder()
                                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                                    .body(Full::new(err.to_string().into()))
                            }
                        }
                    });

                    if let Err(err) = http1::Builder::new()
                        .serve_connection(TokioIo::new(stream), service)
                        .await
                    {
                        panic!("failed to serve connection: {err}")
                    }
                });
            }
        }
    */
}
