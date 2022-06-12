use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::ops::Sub;
use std::time::Duration;

use bytes::Bytes;
use chrono::Utc;
use event::Metric;
use framework::config::{
    default_interval, deserialize_duration, serialize_duration, DataType, GenerateConfig, Output,
    SourceConfig, SourceContext, SourceDescription,
};
use framework::http::{Auth, HttpClient};
use framework::tls::{MaybeTlsSettings, TlsConfig};
use framework::Source;
use futures::{StreamExt, TryFutureExt};
use hyper::{StatusCode, Uri};
use nom::{
    bytes::complete::{tag, take_while_m_n},
    combinator::{all_consuming, map_res},
    error::ErrorKind,
    sequence::{preceded, terminated, tuple},
};
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};
use tokio_stream::wrappers::IntervalStream;

#[derive(Debug, Deserialize, Serialize)]
struct NginxStubConfig {
    endpoints: Vec<String>,
    #[serde(default = "default_interval")]
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "serialize_duration"
    )]
    interval: Duration,
    tls: Option<TlsConfig>,
    auth: Option<Auth>,
}

impl GenerateConfig for NginxStubConfig {
    fn generate_config() -> String {
        format!(
            r#"
# HTTP/HTTPS endpoint to Consul server.
endpoints:
- http://localhost:8500

# The interval between scrapes.
#
# interval: 15s

# Configures the TLS options for outgoing connections.
# tls:
{}

# Configures the authentication strategy.
# auth:
{}
"#,
            TlsConfig::generate_commented_with_indent(2),
            Auth::generate_commented_with_indent(2),
        )
    }
}

inventory::submit! {
    SourceDescription::new::<NginxStubConfig>("nginx_stub")
}

#[async_trait::async_trait]
#[typetag::serde(name = "nginx_stub")]
impl SourceConfig for NginxStubConfig {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let tls = MaybeTlsSettings::from_config(&self.tls, false)?;
        let http_client = HttpClient::new(tls, &cx.proxy)?;

        let mut sources = Vec::with_capacity(self.endpoints.len());
        for endpoint in self.endpoints.iter() {
            sources.push(NginxStub::new(
                http_client.clone(),
                endpoint.clone(),
                self.auth.clone(),
            )?);
        }

        let mut output = cx.output;
        let interval = tokio::time::interval(self.interval);
        let mut ticker = IntervalStream::new(interval).take_until(cx.shutdown);

        Ok(Box::pin(async move {
            while ticker.next().await.is_some() {
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
        vec![Output::default(DataType::Metric)]
    }

    fn source_type(&self) -> &'static str {
        "nginx_stub"
    }
}

#[derive(Debug, Snafu)]
enum NginxBuildError {
    #[snafu(display("Failed to parse endpoint: {}", source))]
    HostInvalidUri { source: http::uri::InvalidUri },
}

#[derive(Debug, Snafu)]
enum NginxError {
    #[snafu(display("Invalid response status: {}", status))]
    InvalidResponseStatus { status: StatusCode },
}

#[derive(Debug)]
struct NginxStub {
    client: HttpClient,
    endpoint: String,
    auth: Option<Auth>,
    tags: BTreeMap<String, String>,
}

impl NginxStub {
    fn new(client: HttpClient, endpoint: String, auth: Option<Auth>) -> Result<Self, crate::Error> {
        let mut tags = BTreeMap::new();
        tags.insert("endpoint".into(), endpoint.clone());
        tags.insert("host".into(), Self::get_endpoint_host(&endpoint)?);

        Ok(Self {
            client,
            endpoint,
            auth,
            tags,
        })
    }

    fn get_endpoint_host(endpoint: &str) -> crate::Result<String> {
        let uri: Uri = endpoint.parse().context(HostInvalidUriSnafu)?;

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
        let resp = self.get_nginx_resp().await?;

        let status = NginxStubStatus::try_from(String::from_utf8_lossy(&resp).as_ref())?;

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

    async fn get_nginx_resp(&self) -> crate::Result<Bytes> {
        let mut req = http::Request::get(&self.endpoint).body(hyper::Body::empty())?;
        if let Some(auth) = &self.auth {
            auth.apply(&mut req);
        }

        let resp = self.client.send(req).await?;
        let (parts, body) = resp.into_parts();
        match parts.status {
            StatusCode::OK => hyper::body::to_bytes(body).err_into().await,
            status => Err(Box::new(NginxError::InvalidResponseStatus { status })),
        }
    }
}

#[derive(Debug, Snafu, PartialEq)]
enum ParseError {
    #[snafu(display("failed to parse nginx stub status, kind: {:?}", kind))]
    NginxStubStatusParseError { kind: ErrorKind },
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

fn get_u64(input: &str) -> nom::IResult<&str, u64, nom::error::Error<&str>> {
    map_res(
        take_while_m_n(1, 20, |c: char| c.is_digit(10)),
        |s: &str| s.parse::<u64>(),
    )(input)
}

impl<'a> TryFrom<&'a str> for NginxStubStatus {
    type Error = ParseError;

    // The `ngx_http_stub_status_module` response:
    // https://github.com/nginx/nginx/blob/master/src/http/modules/ngx_http_stub_status_module.c#L137-L145
    fn try_from(input: &'a str) -> Result<Self, Self::Error> {
        // `usize::MAX` eq `18446744073709551615` (20 char)
        match all_consuming(tuple((
            preceded(tag("Active connections: "), get_u64),
            preceded(tag(" \nserver accepts handled requests\n "), get_u64),
            preceded(tag(" "), get_u64),
            preceded(tag(" "), get_u64),
            preceded(tag(" \nReading: "), get_u64),
            preceded(tag(" Writing: "), get_u64),
            terminated(preceded(tag(" Waiting: "), get_u64), tag(" \n")),
        )))(input)
        {
            Ok((_, (active, accepts, handled, requests, reading, writing, waiting))) => {
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

            Err(err) => match err {
                nom::Err::Error(err) => {
                    Err(ParseError::NginxStubStatusParseError { kind: err.code })
                }

                nom::Err::Incomplete(_) | nom::Err::Failure(_) => unreachable!(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<NginxStubConfig>()
    }

    #[test]
    fn nginx_stub_status_try_from() {
        let input = "Active connections: 291 \n\
                    server accepts handled requests\n \
                    16630948 16630948 31070465 \n\
                    Reading: 6 Writing: 179 Waiting: 106 \n";

        assert_eq!(
            NginxStubStatus::try_from(input).expect("valid data"),
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
    mod nginx {
        use std::collections::HashMap;
        use testcontainers::{Container, Docker, Image, WaitForMessage};

        const CONTAINER_IDENTIFIER: &str = "nginx";
        const DEFAULT_TAG: &str = "1.21.3";

        #[derive(Debug, Default, Clone)]
        pub struct NginxArgs;

        impl IntoIterator for NginxArgs {
            type Item = String;
            type IntoIter = ::std::vec::IntoIter<String>;

            fn into_iter(self) -> Self::IntoIter {
                vec![].into_iter()
            }
        }

        #[derive(Debug)]
        pub struct Nginx {
            tag: String,
            arguments: NginxArgs,
            envs: HashMap<String, String>,
            pub volumes: HashMap<String, String>,
        }

        impl Default for Nginx {
            fn default() -> Self {
                Self {
                    tag: DEFAULT_TAG.to_string(),
                    arguments: NginxArgs,
                    envs: HashMap::new(),
                    volumes: HashMap::new(),
                }
            }
        }

        impl Image for Nginx {
            type Args = NginxArgs;
            type EnvVars = HashMap<String, String>;
            type Volumes = HashMap<String, String>;
            type EntryPoint = std::convert::Infallible;

            fn descriptor(&self) -> String {
                format!("{}:{}", CONTAINER_IDENTIFIER, &self.tag)
            }

            fn wait_until_ready<D: Docker>(&self, container: &Container<'_, D, Self>) {
                container
                    .logs()
                    .stdout
                    .wait_for_message("worker process")
                    .unwrap();
            }

            fn args(&self) -> Self::Args {
                self.arguments.clone()
            }

            fn env_vars(&self) -> Self::EnvVars {
                self.envs.clone()
            }

            fn volumes(&self) -> Self::Volumes {
                self.volumes.clone()
            }

            fn with_args(self, arguments: Self::Args) -> Self {
                Nginx { arguments, ..self }
            }
        }
    }

    use super::NginxStubStatus;
    use framework::config::ProxyConfig;
    use framework::http::{Auth, HttpClient};
    use hyper::{Body, StatusCode, Uri};
    use nginx::Nginx;
    use std::convert::TryInto;
    use testcontainers::Docker;

    async fn test_nginx(path: &'static str, auth: Option<Auth>, proxy: ProxyConfig) {
        let docker = testcontainers::clients::Cli::default();
        let mut image = Nginx::default();
        let pwd = std::env::current_dir().unwrap();
        image.volumes.insert(
            format!("{}/tests/fixtures/nginx/nginx.conf", pwd.to_string_lossy()),
            "/etc/nginx/nginx.conf".to_string(),
        );
        image.volumes.insert(
            format!(
                "{}/tests/fixtures/nginx/nginx_auth_basic.conf",
                pwd.to_string_lossy()
            ),
            "/etc/nginx/nginx_auth_basic.conf".to_string(),
        );
        let service = docker.run(image);
        let host_port = service.get_host_port(80).unwrap();
        let uri = format!("http://127.0.0.1:{}{}", host_port, path)
            .parse::<Uri>()
            .unwrap();

        let cli = HttpClient::new(None, &proxy.clone()).unwrap();
        let mut req = http::Request::get(uri).body(Body::empty()).unwrap();

        if let Some(auth) = auth {
            auth.apply(&mut req);
        }

        let resp = cli.send(req).await.unwrap();

        let (parts, body) = resp.into_parts();
        assert_eq!(parts.status, StatusCode::OK);
        let s = hyper::body::to_bytes(body).await.unwrap();

        let s = std::str::from_utf8(&s).unwrap();
        let _status: NginxStubStatus = s.try_into().unwrap();
    }

    #[tokio::test]
    async fn test_nginx_stub_status() {
        test_nginx("/basic_status", None, ProxyConfig::default()).await
    }

    #[tokio::test]
    async fn test_nginx_stub_status_with_auth() {
        test_nginx(
            "/basic_status_auth",
            Some(Auth::Basic {
                user: "tom".to_string(),
                password: "123456".to_string(),
            }),
            ProxyConfig::default(),
        )
        .await
    }
}
