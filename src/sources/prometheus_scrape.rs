use std::time::Instant;
use futures::{FutureExt, SinkExt, StreamExt, TryFutureExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use tokio_stream::wrappers::IntervalStream;

use crate::http::{Auth, HTTPClient};
use crate::tls::{TLSConfig, TLSSettings};
use crate::config::{serialize_duration, deserialize_duration, default_interval, default_false, SourceConfig, SourceContext, DataType, ticker_from_duration, ProxyConfig};
use crate::pipeline::Pipeline;
use crate::shutdown::ShutdownSignal;
use crate::sources::Source;


// pulled up, and split over multiple lines, because the long lines trip up rustfmt such that it
// gave up trying to format, but reported no error
static PARSE_ERROR_NO_PATH: &str = "No path is set on the endpoint and we got a parse error,\
                                    did you mean to use /metrics? This behavior changed in version 0.11.";
static NOT_FOUND_NO_PATH: &str = "No path is set on the endpoint and we got a 404,\
                                  did you mean to use /metrics?\
                                  This behavior changed in version 0.11.";

#[derive(Debug, Deserialize, Serialize)]
struct PrometheusScrapeConfig {
    endpoints: Vec<String>,
    #[serde(default = "default_interval")]
    #[serde(serialize_with = "serialize_duration", deserialize_with = "deserialize_duration")]
    interval: chrono::Duration,
    #[serde(default = "default_false")]
    honor_labels: bool,
    tls: Option<TLSConfig>,
    auth: Option<Auth>,
}

#[async_trait::async_trait]
#[typetag::serde(name = "prometheus_scrape")]
impl SourceConfig for PrometheusScrapeConfig {
    async fn build(&self, ctx: SourceContext) -> crate::Result<Source> {
        let urls = self.endpoints
            .iter()
            .map(|s| s.parse::<http::Uri>().context(crate::sources::UriParseError))
            .collect::<Result<Vec<http::Uri>, crate::sources::BuildError>>()?;
        let tls = TLSSettings::from_config(&self.tls)?;
        Ok(scrape(
            urls,
            tls,
            self.auth.clone(),
            ctx.proxy,
            None,
            self.honor_labels,
            self.interval,
            ctx.shutdown,
            ctx.out,
        ))
    }

    fn output_type(&self) -> DataType {
        DataType::Metric
    }

    fn source_type(&self) -> &'static str {
        "prometheus_scrape"
    }
}

fn scrape(
    urls: Vec<http::Uri>,
    tls: TLSSettings,
    auth: Option<Auth>,
    proxy: ProxyConfig,
    instance_tag: Option<String>,
    honor_labels: bool,
    interval: chrono::Duration,
    shutdown: ShutdownSignal,
    output: Pipeline,
) -> Source {
    let output = output.sink_map_err(|err| {
        error!(
            message = "Error sending metric",
            %err
        );
    });

    let ticker = ticker_from_duration(interval).unwrap();

    Box::pin(
        ticker.take_until(shutdown)
            .map(move |_| futures::stream::iter(urls.clone()))
            .flatten()
            .map(move |url| {
                let client = HTTPClient::new(tls, &proxy)
                    .expect("Building HTTP client failed");
                let mut req = http::Request::get(&url)
                    .body(hyper::body::Body::empty())
                    .expect("error creating request");
                if let Some(auth) = &auth {
                    auth.apply(&mut req);
                }

                let instance = instance_tag.as_ref().map(|tag| {
                    let instance = format!(
                        "{}:{}",
                        url.host().unwrap_or_default(),
                        url.port_u16().unwrap_or_else(|| match url.scheme() {
                            Some(scheme) if scheme == &http::uri::Scheme::HTTP => 80,
                            Some(scheme) if scheme == &http::uri::Scheme::HTTPS => 443,
                            _ => 0
                        })
                    );
                });

                let start = Instant::now();
                client.send(req)
                    .map_err(crate::Error::from)
                    .and_then(|resp| async move {
                        let (header, body) = resp.into_parts();
                        let body = hyper::body::to_bytes(body).await?;
                        Ok((header, body))
                    })
                    .into_stream()
                    .filter_map(move |resp| {
                        std::future::ready(match resp {
                            Ok((header, body)) if header.status == hyper::StatusCode::OK => {
                                match prometheus::parse_text(&body) {
                                    Ok(groups) => {
                                        // TODO: convert
                                    }
                                    Err(err) => {
                                        // TODO: handle it
                                    }
                                }
                            }

                            Ok((header, _)) => {
                                if header.status == hyper::StatusCode::NOT_FOUND && url.path() == "/" {
                                    warn!(
                                        message = NOT_FOUND_NO_PATH,
                                        endpoint = %url
                                    );
                                }

                                None
                            }

                            Err(err) => {
                                None
                            }
                        })
                    })
            })
            .flatten()
            .forward(output)
            .inspect(|_| info!("Finished sending"))
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dummy() {

    }
}