use std::sync::Arc;
use std::time::{Duration, Instant};

use configurable::configurable_component;
use event::Metric;
use framework::Source;
use framework::config::{OutputType, SourceConfig, SourceContext, default_interval};
use framework::http::{Auth, HttpClient, HttpError};
use framework::tls::TlsConfig;
use http::{StatusCode, Uri};
use http_body_util::{BodyExt, Full};
use thiserror::Error;
use tokio::task::JoinSet;

use crate::common::calculate_start_with_jitter;
use crate::common::prometheus::convert_metrics;

/// Collect metrics from prometheus clients.
#[configurable_component(source, name = "prometheus_scrape")]
#[serde(deny_unknown_fields)]
struct Config {
    /// Endpoints to scrape metrics from.
    #[configurable(required, format = "uri", example = "http://example.com/metrics")]
    targets: Vec<String>,

    /// Duration between each scrape.
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,

    /// Controls how tag conflicts are handled if the scraped source has tags
    /// that Vertex would add. If true Vertex will not add the new tag if the
    /// scraped metric has the tag already. If false, Vertex will rename the
    /// conflicting tag by adding "exported_" to it. This matches Prometheus's
    /// "honor_labels" configuration.
    #[serde(default)]
    honor_labels: bool,

    tls: Option<TlsConfig>,

    auth: Option<Auth>,

    /// Global jitterSeed seed is used to spread scrape workload across HA setup.
    #[serde(default)]
    jitter_seed: u64,
}

#[async_trait::async_trait]
#[typetag::serde(name = "prometheus_scrape")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let targets = self
            .targets
            .iter()
            .map(|s| s.parse::<Uri>())
            .collect::<Result<Vec<Uri>, _>>()?;
        let client = HttpClient::new(self.tls.as_ref(), &cx.proxy)?;

        let shutdown = cx.shutdown;
        let output = cx.output;
        let auth = Arc::new(self.auth.clone());
        let interval = self.interval;
        let jitter_seed = self.jitter_seed;
        let honor_labels = self.honor_labels;

        Ok(Box::pin(async move {
            let mut set = JoinSet::new();

            for target in targets {
                let mut output = output.clone();
                let mut shutdown = shutdown.clone();

                let client = client.clone();
                let auth = Arc::clone(&auth);
                let instance = format!(
                    "{}:{}",
                    target.host().unwrap_or_default(),
                    target.port_u16().unwrap_or_else(|| match target.scheme() {
                        Some(scheme) if scheme == &http::uri::Scheme::HTTP => 80,
                        Some(scheme) if scheme == &http::uri::Scheme::HTTPS => 443,
                        _ => 0,
                    })
                );

                set.spawn(async move {
                    let start = calculate_start_with_jitter(&target, interval, jitter_seed);
                    let mut ticker = tokio::time::interval_at(start.into(), interval);

                    loop {
                        tokio::select! {
                            _ = &mut shutdown => break,
                            _ = ticker.tick() => {}
                        }

                        let start = Instant::now();
                        let result = scrape_one(&client, auth.as_ref(), &target).await;
                        let elapsed = start.elapsed();

                        let (mut metrics, success) = match result {
                            Ok(metrics) => {
                                if metrics.is_empty() {
                                    warn!(
                                        message = "cannot read or parse metrics",
                                        instance,
                                        internal_log_rate_limit = 60
                                    );
                                }

                                (metrics, true)
                            }
                            Err(err) => {
                                warn!(
                                    message = "scrape metrics failed",
                                    %err,
                                    instance,
                                );

                                (vec![], false)
                            }
                        };

                        metrics.extend([
                            Metric::gauge("up", "", success),
                            Metric::gauge("scrape_duration_seconds", "", elapsed),
                            Metric::gauge("scrape_samples_scraped", "", metrics.len()),
                        ]);

                        // NOTE: timestamp already set in the conversion function, so we don't
                        // need to set it here
                        metrics.iter_mut().for_each(|metric| {
                            // Handle "instance" overwrite
                            let tags = metric.tags_mut();

                            if let Some(value) = tags.remove("instance")
                                && honor_labels
                            {
                                tags.insert("instance", value)
                            }

                            metric.insert_tag("instance", instance.clone());
                        });

                        if let Err(err) = output.send_batch(metrics).await {
                            error!(
                                message = "Error sending scraped metrics",
                                %err
                            );

                            return;
                        }
                    }
                });
            }

            set.join_all().await;

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

#[derive(Debug, Error)]
enum ScrapeError {
    #[error("http error, {0}")]
    Http(#[from] HttpError),

    #[error("unexpected status code {0}")]
    UnexpectedStatusCode(StatusCode),

    #[error("parse metrics failed {0}")]
    Parse(prometheus::Error),
}

async fn scrape_one(
    client: &HttpClient,
    auth: &Option<Auth>,
    uri: &Uri,
) -> Result<Vec<Metric>, ScrapeError> {
    let mut req = http::Request::get(uri)
        .body(Full::default())
        .map_err(HttpError::BuildRequest)?;
    if let Some(auth) = auth {
        auth.apply(&mut req);
    }

    let resp = client.send(req).await.map_err(ScrapeError::Http)?;

    let (header, incoming) = resp.into_parts();
    if header.status != StatusCode::OK {
        return Err(ScrapeError::UnexpectedStatusCode(header.status));
    }

    let data = incoming
        .collect()
        .await
        .map_err(|err| ScrapeError::Http(HttpError::ReadIncoming(err)))?
        .to_bytes();
    let body = String::from_utf8_lossy(&data);

    let metrics = prometheus::parse_text(&body).map_err(ScrapeError::Parse)?;

    Ok(convert_metrics(metrics))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>();
    }
}
