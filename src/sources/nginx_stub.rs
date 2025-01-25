use std::num::ParseIntError;
use std::ops::Sub;
use std::time::Duration;

use bytes::Bytes;
use chrono::Utc;
use configurable::configurable_component;
use event::tags::Tags;
use event::{tags, Metric};
use framework::config::{default_interval, Output, SourceConfig, SourceContext};
use framework::http::{Auth, HttpClient, HttpError};
use framework::tls::TlsConfig;
use framework::Source;
use http_body_util::{BodyExt, Full};
use hyper::{StatusCode, Uri};
use thiserror::Error;

#[configurable_component(source, name = "nginx_stub")]
struct Config {
    /// HTTP/HTTPS endpoint to Nginx server.
    ///
    /// http://nginx.org/en/docs/http/ngx_http_stub_status_module.html
    #[configurable(required, format = "uri", example = "http://127.0.0.1:8080/nginx_stub")]
    endpoints: Vec<String>,

    /// Duration between each scrape.
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,

    /// Configures the TLS options for outgoing connections.
    tls: Option<TlsConfig>,

    /// Configures the authentication strategy.
    auth: Option<Auth>,
}

#[async_trait::async_trait]
#[typetag::serde(name = "nginx_stub")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let client = HttpClient::new(self.tls.as_ref(), &cx.proxy)?;
        let mut sources = Vec::with_capacity(self.endpoints.len());
        for endpoint in self.endpoints.iter() {
            sources.push(NginxStub::new(
                client.clone(),
                endpoint.clone(),
                self.auth.clone(),
            )?);
        }

        let SourceContext {
            mut output,
            mut shutdown,
            ..
        } = cx;
        let mut ticker = tokio::time::interval(self.interval);

        Ok(Box::pin(async move {
            loop {
                tokio::select! {
                    biased;

                    _ = &mut shutdown => break,
                    _ = ticker.tick() => {}
                }

                let mut metrics = futures::future::join_all(sources.iter().map(|s| s.collect()))
                    .await
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>();

                let now = Utc::now();
                metrics
                    .iter_mut()
                    .for_each(|metric| metric.timestamp = Some(now));

                if let Err(err) = output.send(metrics).await {
                    error!(
                        message = "Error sending nginx stub metrics",
                        %err
                    );

                    return Err(());
                }
            }

            Ok(())
        }))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::metrics()]
    }

    fn can_acknowledge(&self) -> bool {
        false
    }
}

#[derive(Debug, Error)]
enum NginxError {
    #[error("build http request failed: {0}")]
    Http(#[from] http::Error),

    #[error("send http request failed: {0}")]
    Request(#[from] HttpError),

    #[error("invalid response status: {0}")]
    InvalidResponseStatus(StatusCode),

    #[error("failed to parse {field}, err: {err}")]
    Parse {
        err: ParseIntError,
        field: &'static str,
    },
}

#[derive(Debug)]
struct NginxStub {
    client: HttpClient,
    endpoint: String,
    auth: Option<Auth>,
    tags: Tags,
}

impl NginxStub {
    fn new(client: HttpClient, endpoint: String, auth: Option<Auth>) -> Result<Self, crate::Error> {
        let tags = tags!(
            "endpoint" => endpoint.clone(),
            "host" => Self::get_endpoint_host(&endpoint)?
        );

        Ok(Self {
            client,
            endpoint,
            auth,
            tags,
        })
    }

    fn get_endpoint_host(endpoint: &str) -> crate::Result<String> {
        let uri: Uri = endpoint.parse()?;

        let host = match (uri.host().unwrap_or(""), uri.port()) {
            (host, None) => host.to_owned(),
            (host, Some(port)) => format!("{}:{}", host, port),
        };

        Ok(host)
    }

    async fn collect(&self) -> Vec<Metric> {
        let start = Utc::now();
        let (up, mut metrics) = match self.collect_metrics().await {
            Ok(metrics) => (1.0, metrics),
            Err(_) => (0.0, vec![]),
        };
        let end = Utc::now();
        let d = end
            .sub(start)
            .num_nanoseconds()
            .expect("Nano seconds should not overflow");

        metrics.push(Metric::gauge_with_tags(
            "nginx_up",
            "",
            up,
            self.tags.clone(),
        ));
        metrics.push(Metric::gauge_with_tags(
            "nginx_scrape_duration_seconds",
            "",
            d as f64 / 1000.0 / 1000.0 / 1000.0,
            self.tags.clone(),
        ));

        for m in metrics.iter_mut() {
            m.timestamp = Some(end);
        }

        metrics
    }

    async fn collect_metrics(&self) -> crate::Result<Vec<Metric>> {
        let status = get_stub_status(&self.client, &self.endpoint, self.auth.as_ref()).await?;

        Ok(vec![
            Metric::gauge_with_tags(
                "nginx_connections_active",
                "",
                status.active as f64,
                self.tags.clone(),
            ),
            Metric::sum_with_tags(
                "nginx_connections_accepted_total",
                "",
                status.accepts as f64,
                self.tags.clone(),
            ),
            Metric::sum_with_tags(
                "nginx_connections_handled_total",
                "",
                status.handled as f64,
                self.tags.clone(),
            ),
            Metric::gauge_with_tags(
                "nginx_connections_reading",
                "",
                status.reading as f64,
                self.tags.clone(),
            ),
            Metric::gauge_with_tags(
                "nginx_connections_writing",
                "",
                status.writing as f64,
                self.tags.clone(),
            ),
            Metric::gauge_with_tags(
                "nginx_connections_waiting",
                "",
                status.waiting as f64,
                self.tags.clone(),
            ),
        ])
    }
}

async fn get_stub_status(
    cli: &HttpClient,
    uri: &str,
    auth: Option<&Auth>,
) -> Result<NginxStubStatus, NginxError> {
    let mut req = http::Request::get(uri).body(Full::default())?;
    if let Some(auth) = auth {
        auth.apply(&mut req);
    }

    let resp = cli.send(req).await?;
    let (parts, incoming) = resp.into_parts();
    let body: Bytes = match parts.status {
        StatusCode::OK => incoming
            .collect()
            .await
            .map_err(|err| NginxError::Request(HttpError::ReadIncoming(err)))?
            .to_bytes(),
        status => return Err(NginxError::InvalidResponseStatus(status)),
    };

    parse(String::from_utf8_lossy(&body).as_ref())
}

#[derive(Debug, PartialEq, Eq)]
struct NginxStubStatus {
    active: u64,
    accepts: u64,
    handled: u64,
    requests: u64,
    reading: u64,
    writing: u64,
    waiting: u64,
}

// The `ngx_http_stub_status_module` response:
// https://github.com/nginx/nginx/blob/master/src/http/modules/ngx_http_stub_status_module.c#L137-L145
fn parse(input: &str) -> Result<NginxStubStatus, NginxError> {
    let parts = input.split_ascii_whitespace().collect::<Vec<_>>();

    let active = parts[2].parse().map_err(|err| NginxError::Parse {
        err,
        field: "active",
    })?;
    let accepts = parts[7].parse().map_err(|err| NginxError::Parse {
        err,
        field: "accepts",
    })?;
    let handled = parts[8].parse().map_err(|err| NginxError::Parse {
        err,
        field: "handled",
    })?;
    let requests = parts[9].parse().map_err(|err| NginxError::Parse {
        err,
        field: "requests",
    })?;
    let reading = parts[11].parse().map_err(|err| NginxError::Parse {
        err,
        field: "reading",
    })?;
    let writing = parts[13].parse().map_err(|err| NginxError::Parse {
        err,
        field: "writing",
    })?;
    let waiting = parts[15].parse().map_err(|err| NginxError::Parse {
        err,
        field: "waiting",
    })?;

    Ok(NginxStubStatus {
        active,
        accepts,
        handled,
        requests,
        reading,
        writing,
        waiting,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>()
    }

    #[test]
    fn nginx_stub_status_try_from() {
        let input = "Active connections: 291 \n\
                    server accepts handled requests\n \
                    16630948 16630948 31070465 \n\
                    Reading: 6 Writing: 179 Waiting: 106 \n";

        assert_eq!(
            parse(input).expect("valid data"),
            NginxStubStatus {
                active: 291,
                accepts: 16630948,
                handled: 16630948,
                requests: 31070465,
                reading: 6,
                writing: 179,
                waiting: 106,
            }
        )
    }
}

#[cfg(all(test, feature = "integration-tests-nginx_stub"))]
mod integration_tests {
    use std::env::current_dir;

    use framework::config::ProxyConfig;
    use framework::http::{Auth, HttpClient};

    use super::get_stub_status;
    use crate::testing::ContainerBuilder;

    #[tokio::test]
    async fn new_test_nginx() {
        let pwd = current_dir().unwrap();
        let container = ContainerBuilder::new("nginx:1.21.3")
            .with_port(80)
            .with_volume(
                format!("{}/tests/nginx/nginx.conf", pwd.to_string_lossy()),
                "/etc/nginx/nginx.conf".to_string(),
            )
            .with_volume(
                format!(
                    "{}/tests/nginx/nginx_auth_basic.conf",
                    pwd.to_string_lossy()
                ),
                "/etc/nginx/nginx_auth_basic.conf".to_string(),
            )
            .run()
            .unwrap();

        container
            .wait(crate::testing::WaitFor::Stdout(" start worker processes"))
            .unwrap();

        let address = container.get_mapped_addr(80);

        let cli = HttpClient::new(None, &ProxyConfig::default()).unwrap();

        // without auth
        let status = get_stub_status(&cli, &format!("http://{}/basic_status", address), None)
            .await
            .unwrap();
        assert_eq!(status.requests, 1);
        assert_eq!(status.active, 1);

        // with auth
        let status = get_stub_status(
            &cli,
            &format!("http://{}/basic_status_auth", address),
            Some(&Auth::Basic {
                user: "tom".to_string(),
                password: "123456".to_string(),
            }),
        )
        .await
        .unwrap();
        assert_eq!(status.requests, 2);
        assert_eq!(status.active, 1);
        assert_eq!(status.accepts, 1);
    }
}
