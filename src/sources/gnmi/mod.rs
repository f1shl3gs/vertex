mod auth;
mod xpath;

mod proto {
    #![allow(clippy::doc_lazy_continuation)]
    #![allow(clippy::enum_variant_names)]
    #![allow(unused_qualifications)]
    #![allow(clippy::trivially_copy_pass_by_ref)]

    include!(concat!(env!("OUT_DIR"), "/gnmi.rs"));

    pub use g_nmi_client::GNmiClient;

    #[cfg(test)]
    impl From<&str> for PathElem {
        fn from(name: &str) -> Self {
            PathElem {
                name: name.to_string(),
                key: Default::default(),
            }
        }
    }
}

mod gnmi_ext {
    #![allow(clippy::trivially_copy_pass_by_ref)]
    #![allow(clippy::enum_variant_names)]

    include!(concat!(env!("OUT_DIR"), "/gnmi_ext.rs"));
}

use std::collections::BTreeMap;
use std::str::FromStr;
use std::time::Duration;

use auth::{Auth, AuthInterceptor};
use configurable::{Configurable, configurable_component};
use event::{Metric, tags};
use framework::config::{OutputType, SourceConfig, SourceContext, default_interval};
use framework::tls::TlsConfig;
use framework::{Pipeline, ShutdownSignal, Source};
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use http::Uri;
use hyper_util::client::legacy::connect::HttpConnector;
use proto::subscribe_response::Response;
use proto::typed_value::Value;
use proto::{CapabilityRequest, Encoding, Notification, PathElem, SubscriptionList};
use proto::{GNmiClient, SubscribeRequest, Subscription, SubscriptionMode};
use rustls::ClientConfig;
use serde::{Deserialize, Serialize};
use tonic::codegen::InterceptedService;
use tonic::transport::Channel;

#[derive(Clone, Configurable, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum SubscriptionModeConfig {
    /// The target sends an update on element value change.
    OnChange,

    /// The target samples values according to the interval.
    #[default]
    Sample,
}

#[derive(Clone, Configurable, Debug, Deserialize, Serialize)]
struct SubscriptionConfig {
    // /// The resource name the path point to, and it will be used as the
    // /// prefix of the metrics too.
    // name: String,
    /// Label to disambiguate path.
    origin: Option<String>,

    /// XPath of the resources
    path: String,

    #[serde(default)]
    mode: SubscriptionModeConfig,

    /// The interval of gnmi server sending samples
    #[serde(default, with = "humanize::duration::serde_option")]
    sample_interval: Option<Duration>,
}

#[configurable_component(source, name = "gnmi")]
struct Config {
    /// Address and port of the gNMI GRPC server
    #[configurable(format = "uri", example = "http://172.20.20.2:57400")]
    endpoints: Vec<String>,

    tls: Option<TlsConfig>,

    /// Auth credentials
    auth: Option<Auth>,

    /// Interval for each target's flush
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,

    subscriptions: Vec<SubscriptionConfig>,
}

#[async_trait::async_trait]
#[typetag::serde(name = "gnmi")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        // validate endpoints
        if self.endpoints.is_empty() {
            return Err("no endpoints specified".into());
        }

        // validate subscriptions
        if self.subscriptions.is_empty() {
            return Err("subscriptions is required".into());
        }
        for subscription in &self.subscriptions {
            if let Err(_err) = xpath::parse(&subscription.path) {
                return Err(format!("invalid subscription path \"{}\"", subscription.path).into());
            }
        }

        let auth = AuthInterceptor::new(self.auth.as_ref())?;
        let endpoints = self
            .endpoints
            .iter()
            .map(|s| Uri::from_str(s.as_str()))
            .collect::<Result<Vec<_>, _>>()?;
        let tls = match self.tls.as_ref() {
            Some(tls) => tls.client_config()?,
            None => TlsConfig::default().client_config()?,
        };

        Ok(Box::pin(run(
            endpoints,
            auth,
            tls,
            self.interval,
            self.subscriptions.clone(),
            cx.output,
            cx.shutdown,
        )))
    }

    fn outputs(&self) -> Vec<OutputType> {
        vec![OutputType::metric()]
    }
}

type GnmiClient = GNmiClient<InterceptedService<Channel, AuthInterceptor>>;

async fn run(
    endpoints: Vec<Uri>,
    auth: AuthInterceptor,
    tls: ClientConfig,
    interval: Duration,
    subscriptions: Vec<SubscriptionConfig>,
    output: Pipeline,
    shutdown: ShutdownSignal,
) -> Result<(), ()> {
    let mut http = HttpConnector::new();
    http.enforce_http(false);
    http.set_connect_timeout(Some(Duration::from_secs(5)));

    let https = hyper_rustls::HttpsConnectorBuilder::new()
        .with_tls_config(tls)
        .https_or_http()
        .enable_http2()
        .wrap_connector(http);

    let mut tasks = FuturesUnordered::new();
    for endpoint in endpoints {
        let endpoint = Channel::builder(endpoint).connect_timeout(Duration::from_secs(5));
        let channel = Channel::new(https.clone(), endpoint);
        let client = GNmiClient::with_interceptor(channel, auth.clone());

        tasks.push(collect(
            client,
            interval,
            &subscriptions,
            output.clone(),
            shutdown.clone(),
        ));
    }

    while tasks.next().await.is_some() {}

    Ok(())
}

const BACKOFF_TIMEOUT: Duration = Duration::from_secs(10);

async fn check_capabilities(client: &mut GnmiClient) -> Option<i32> {
    const SUPPORTED_ENCODING: i32 = Encoding::Proto as i32;

    let capabilities = match client.capabilities(CapabilityRequest::default()).await {
        Ok(resp) => resp.into_inner(),
        Err(status) => {
            warn!(
                message = "get capabilities failed",
                ?status,
                internal_log_rate_secs = 30
            );
            return None;
        }
    };

    debug!(
        message = "get capabilities success",
        supported_encodings = ?capabilities.supported_encodings
    );

    if capabilities
        .supported_encodings
        .contains(&SUPPORTED_ENCODING)
    {
        return Some(SUPPORTED_ENCODING);
    }

    warn!(
        message = "cannot find supported encodings",
        supported = ?capabilities.supported_encodings,
    );

    None
}

async fn collect(
    mut client: GnmiClient,
    interval: Duration,
    subscriptions: &[SubscriptionConfig],
    mut output: Pipeline,
    mut shutdown: ShutdownSignal,
) {
    let mut backoff: Option<Duration> = None;

    loop {
        if let Some(timeout) = backoff.take() {
            tokio::select! {
                _ = tokio::time::sleep(timeout) => {},
                _ = &mut shutdown => return,
            }
        }

        match check_capabilities(&mut client).await {
            Some(encoding) => {
                debug!(
                    message = "check capabilities success",
                    encoding = ?Encoding::try_from(encoding)
                );
            }
            None => {
                backoff = Some(BACKOFF_TIMEOUT);
                continue;
            }
        };

        let req = build_subscribe_request(subscriptions, interval);
        let mut stream = match client.subscribe(futures::stream::iter([req])).await {
            Ok(resp) => resp.into_inner(),
            Err(err) => {
                warn!(message = "subscribe failed", %err);
                backoff = Some(BACKOFF_TIMEOUT);
                continue;
            }
        };

        let mut ticker = tokio::time::interval(interval);
        let mut stats = BTreeMap::<Vec<PathElem>, f64>::new();
        loop {
            let result = tokio::select! {
                _ = &mut shutdown => return,
                result = stream.next() => if let Some(result) = result {
                    result
                } else {
                    break;
                },
                _ = ticker.tick() => {
                    // flush metrics to sink
                    let metrics = stats.iter()
                        .map(|(path, value)| {
                            path_to_metric(path.as_slice(), *value)
                        })
                        .collect::<Vec<_>>();

                    if let Err(_err) = output.send(metrics).await {
                        return
                    }

                    continue
                }
            };

            match result {
                Ok(resp) => {
                    let Some(resp) = resp.response else {
                        continue;
                    };

                    match resp {
                        Response::Update(notification) => {
                            apply_update(notification, &mut stats);
                        }
                        Response::SyncResponse(sync) => {
                            debug!(message = "sync response", sync);
                        }
                        // Deprecated in favour of google.golang.org/genproto/googleapis/rpc/status
                        Response::Error(err) => {
                            warn!(message = "error resp received", ?err);
                            break;
                        }
                    }
                }
                Err(status) => {
                    warn!(message = "get subscription failed", ?status);
                    break;
                }
            }
        }

        backoff = Some(BACKOFF_TIMEOUT);
    }
}

fn apply_update(notification: Notification, stats: &mut BTreeMap<Vec<PathElem>, f64>) {
    for path in notification.delete {
        stats.remove(&path.elem);
    }

    for update in notification.update {
        let Some(path) = update.path else {
            continue;
        };

        let Some(val) = update.val else {
            continue;
        };

        let Some(value) = val.value else {
            continue;
        };

        let value = match value {
            Value::IntVal(i) => i as f64,
            Value::UintVal(u) => u as f64,
            Value::FloatVal(f) => f as f64,
            Value::BoolVal(b) => b as u8 as f64,
            Value::DoubleVal(d) => d,
            // Value::DecimalVal(d) => {}
            _ => continue,
        };

        stats.insert(path.elem, value);
    }
}

#[inline]
fn build_subscribe_request(scs: &[SubscriptionConfig], interval: Duration) -> SubscribeRequest {
    let subscription = scs
        .iter()
        .map(|config| {
            let path = xpath::parse(&config.path).unwrap();

            match config.mode {
                SubscriptionModeConfig::Sample => Subscription {
                    path: Some(path),
                    mode: SubscriptionMode::Sample as i32,
                    sample_interval: config.sample_interval.unwrap_or(interval).as_nanos() as u64,
                    ..Default::default()
                },
                SubscriptionModeConfig::OnChange => Subscription {
                    path: Some(path),
                    mode: SubscriptionMode::OnChange as i32,
                    ..Default::default()
                },
            }
        })
        .collect::<Vec<_>>();

    SubscribeRequest {
        request: {
            let list = SubscriptionList {
                // encoding: Encoding::Proto as i32,
                encoding: Encoding::JsonIetf as i32,
                mode: proto::subscription_list::Mode::Stream as i32,
                subscription,
                ..Default::default()
            };

            Some(proto::subscribe_request::Request::Subscribe(list))
        },
        ..Default::default()
    }
}

fn path_to_metric(path: &[PathElem], value: f64) -> Metric {
    let mut name = String::new();
    let mut desc = String::new();
    let mut tags = tags!();

    let mut first = true;
    for segment in path {
        if first {
            first = false;
        } else {
            name.push('_');
        }

        if !segment.name.contains(['-', '/']) {
            name.push_str(segment.name.as_str());
        } else {
            name.push_str(segment.name.replace(['-', '/'], "_").as_str());
        }

        desc.push('/');
        desc.push_str(segment.name.as_str());

        for (key, value) in &segment.key {
            tags.insert(key, value);

            desc.push('[');
            desc.push_str(key);
            desc.push('=');
            desc.push_str(value);
            desc.push(']');
        }
    }

    Metric::gauge_with_tags(name, desc, value, tags)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>()
    }
}
