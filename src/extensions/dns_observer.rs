use std::sync::Arc;
use std::time::Duration;

use configurable::{Configurable, configurable_component};
use framework::Extension;
use framework::config::{ExtensionConfig, ExtensionContext};
use framework::observe::{Endpoint, Observer, run};
use hickory_resolver::TokioAsyncResolver;
use serde::{Deserialize, Serialize};
use tokio::task::JoinSet;
use value::{Value, value};

const fn default_refresh_interval() -> Duration {
    Duration::from_secs(30)
}

#[derive(Configurable, Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
#[allow(clippy::upper_case_acronyms)]
enum QueryType {
    #[default]
    SRV,
    A,
    AAAA,
    MX,
    NS,
}

#[configurable_component(extension, name = "dns_observer")]
struct Config {
    /// A list of DNS domain names to be queried
    names: Vec<String>,

    /// The type of DNS query to perform.
    query_type: QueryType,

    /// The port number used if the query type is not SRV
    port: u16,

    /// The time after which the provided names are refreshed
    #[serde(
        default = "default_refresh_interval",
        with = "humanize::duration::serde"
    )]
    interval: Duration,
}

#[async_trait::async_trait]
#[typetag::serde(name = "dns_observer")]
impl ExtensionConfig for Config {
    async fn build(&self, cx: ExtensionContext) -> crate::Result<Extension> {
        let resolver = Arc::new(TokioAsyncResolver::tokio_from_system_conf()?);

        let observer = Observer::register(cx.name);
        let port = self.port;
        let query_type = self.query_type;
        let names = self.names.clone();

        Ok(Box::pin(run(
            observer,
            self.interval,
            cx.shutdown,
            async move || {
                let mut tasks = JoinSet::new();

                for name in names.iter() {
                    let resolver = Arc::clone(&resolver);

                    tasks.spawn(query(resolver, name.clone(), query_type, port));
                }

                let endpoints = tasks
                    .join_all()
                    .await
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>();

                Ok(endpoints)
            },
        )))
    }
}

async fn query(
    resolver: Arc<TokioAsyncResolver>,
    name: String,
    query_type: QueryType,
    default_port: u16,
) -> Vec<Endpoint> {
    match query_type {
        QueryType::SRV => match resolver.srv_lookup(&name).await {
            Ok(iter) => iter
                .into_iter()
                .map(|srv| {
                    let priority = srv.priority();
                    let port = srv.port();
                    let weight = srv.weight();
                    let target = srv.target();

                    let id = format!("SRV_{}_{}:{}", name, target, port);
                    let target = format!("{}:{}", target, port);
                    let details_target = srv.target().to_string();
                    let details = value!({
                        "priority": priority,
                        "port": port,
                        "weight": weight,
                        "target": details_target,
                    });

                    Endpoint {
                        id,
                        typ: "SRV".to_string(),
                        target,
                        details,
                    }
                })
                .collect::<Vec<_>>(),
            Err(err) => {
                warn!(message = "Failed to lookup SRV records", name, ?err);
                vec![]
            }
        },
        QueryType::A => match resolver.ipv4_lookup(&name).await {
            Ok(iter) => iter
                .into_iter()
                .map(|record| {
                    let id = format!("A_{}_{}", record.0, default_port);
                    let target = format!("{}:{}", record.0, default_port);

                    Endpoint {
                        id,
                        typ: "A".to_string(),
                        target,
                        details: Value::Null,
                    }
                })
                .collect(),
            Err(err) => {
                warn!(message = "Failed to lookup A records", name, ?err);
                vec![]
            }
        },
        QueryType::AAAA => match resolver.ipv6_lookup(&name).await {
            Ok(iter) => iter
                .into_iter()
                .map(|record| {
                    let id = format!("AAAA_{}_{}", record.0, default_port);
                    let target = format!("{}:{}", record.0, default_port);

                    Endpoint {
                        id,
                        typ: "AAAA".to_string(),
                        target,
                        details: Value::Null,
                    }
                })
                .collect(),
            Err(err) => {
                warn!(message = "Failed to look AAAA records", name, ?err);
                vec![]
            }
        },
        QueryType::MX => match resolver.mx_lookup(&name).await {
            Ok(iter) => iter
                .into_iter()
                .map(|mx| {
                    let id = format!("MX_{}_{}:{}", name, mx.exchange(), default_port);
                    let target = format!("{}:{}", mx.exchange(), default_port);
                    let preference = mx.preference();
                    let exchange = mx.exchange().to_string();

                    Endpoint {
                        id,
                        typ: "MX".to_string(),
                        target,
                        details: value!({
                            "preference": preference,
                            "exchange": exchange
                        }),
                    }
                })
                .collect(),
            Err(err) => {
                warn!(message = "Failed to lookup MX records", name, ?err);
                vec![]
            }
        },
        QueryType::NS => match resolver.ns_lookup(&name).await {
            Ok(iter) => iter
                .into_iter()
                .map(|ns| {
                    let id = format!("NS_{}_{}:{}", name, ns.0, default_port);
                    let target = format!("{}:{}", ns.0, default_port);
                    let ns = ns.0.to_string();

                    Endpoint {
                        id,
                        typ: "NS".to_string(),
                        target,
                        details: value!({
                            "ns": ns
                        }),
                    }
                })
                .collect(),
            Err(err) => {
                warn!(message = "Failed to lookup NS records", name, ?err);
                vec![]
            }
        },
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
