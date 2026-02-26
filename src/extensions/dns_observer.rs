use std::sync::Arc;
use std::time::Duration;

use configurable::{Configurable, configurable_component};
use framework::Extension;
use framework::config::{ExtensionConfig, ExtensionContext};
use framework::observe::{Endpoint, Observer, run};
use resolver::{RecordClass, RecordData, RecordType, Resolver};
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
    #[serde(default)]
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
        let resolver = Arc::new(Resolver::with_defaults()?);

        let observer = Observer::register(cx.key);
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
    resolver: Arc<Resolver>,
    name: String,
    query_type: QueryType,
    default_port: u16,
) -> Vec<Endpoint> {
    match query_type {
        QueryType::SRV => match resolver
            .lookup(&name, RecordType::SRV, RecordClass::INET)
            .await
        {
            Ok(msg) => msg
                .answers
                .into_iter()
                .filter_map(|record| match record.data {
                    RecordData::SRV {
                        priority,
                        weight,
                        port,
                        target,
                    } => {
                        let target = String::from_utf8_lossy(&target);

                        let id = format!("SRV_{name}_{target}:{port}");
                        let details = value!({
                            "priority": priority,
                            "port": port,
                            "weight": weight,
                            "target": target.to_string(),
                        });

                        Some(Endpoint {
                            id,
                            typ: "SRV".into(),
                            target: format!("{target}:{port}"),
                            details,
                        })
                    }
                    _ => None,
                })
                .collect::<Vec<_>>(),
            Err(err) => {
                warn!(message = "Failed to lookup SRV records", name, ?err);
                vec![]
            }
        },
        QueryType::A => match resolver
            .lookup(&name, RecordType::A, RecordClass::INET)
            .await
        {
            Ok(msg) => msg
                .answers
                .into_iter()
                .filter_map(|record| match record.data {
                    RecordData::A(ip) => {
                        let id = format!("A_{ip}_{default_port}");
                        let target = format!("{ip}:{default_port}");

                        Some(Endpoint {
                            id,
                            typ: "A".into(),
                            target,
                            details: Value::Null,
                        })
                    }
                    _ => None,
                })
                .collect(),
            Err(err) => {
                warn!(message = "Failed to lookup A records", name, ?err);
                vec![]
            }
        },
        QueryType::AAAA => match resolver
            .lookup(&name, RecordType::A, RecordClass::INET)
            .await
        {
            Ok(msg) => msg
                .answers
                .into_iter()
                .filter_map(|record| match record.data {
                    RecordData::AAAA(ip) => {
                        let id = format!("AAAA_{ip}_{default_port}");
                        let target = format!("{ip}:{default_port}");

                        Some(Endpoint {
                            id,
                            typ: "AAAA".into(),
                            target,
                            details: Value::Null,
                        })
                    }
                    _ => None,
                })
                .collect(),
            Err(err) => {
                warn!(message = "Failed to look AAAA records", name, ?err);
                vec![]
            }
        },
        QueryType::MX => match resolver
            .lookup(&name, RecordType::MX, RecordClass::INET)
            .await
        {
            Ok(msg) => msg
                .answers
                .into_iter()
                .filter_map(|record| match record.data {
                    RecordData::MX {
                        preference,
                        exchange,
                    } => {
                        let exchange = String::from_utf8_lossy(&exchange).to_string();

                        let id = format!("MX_{name}_{exchange}:{default_port}");
                        let target = format!("{exchange}:{default_port}");

                        Some(Endpoint {
                            id,
                            typ: "MX".into(),
                            target,
                            details: value!({
                                "preference": preference,
                                "exchange": exchange
                            }),
                        })
                    }
                    _ => None,
                })
                .collect(),
            Err(err) => {
                warn!(message = "Failed to lookup MX records", name, ?err);
                vec![]
            }
        },
        QueryType::NS => match resolver
            .lookup(&name, RecordType::NS, RecordClass::INET)
            .await
        {
            Ok(msg) => msg
                .answers
                .into_iter()
                .filter_map(|record| match record.data {
                    RecordData::NS(ns) => {
                        let ns = String::from_utf8_lossy(&ns).to_string();
                        let id = format!("NS_{name}_{ns}:{default_port}");
                        let target = format!("{ns}:{default_port}");

                        Some(Endpoint {
                            id,
                            typ: "NS".into(),
                            target,
                            details: value!({ "ns": ns }),
                        })
                    }
                    _ => None,
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
