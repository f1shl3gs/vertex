mod client;

use event::{tags, Event, Metric};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::Instant;
use tokio_stream::wrappers::IntervalStream;

use crate::config::{
    default_std_interval, default_true, deserialize_std_duration, serialize_std_duration, DataType,
    GenerateConfig, SourceConfig, SourceContext, SourceDescription,
};
use crate::http::HttpClient;
use crate::sources::consul::client::{Client, ConsulError};
use crate::sources::Source;
use crate::tls::{MaybeTlsSettings, TlsConfig};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ConsulSourceConfig {
    #[serde(default)]
    tls: Option<TlsConfig>,

    endpoints: Vec<String>,

    #[serde(
        default = "default_std_interval",
        deserialize_with = "deserialize_std_duration",
        serialize_with = "serialize_std_duration"
    )]
    interval: std::time::Duration,

    #[serde(default = "default_true")]
    health_summary: bool,
}

impl GenerateConfig for ConsulSourceConfig {
    fn generate_config() -> serde_yaml::Value {
        serde_yaml::to_value(Self {
            tls: None,
            endpoints: vec!["http://127.0.0.1:8500".to_string()],
            interval: default_std_interval(),
            health_summary: default_true(),
        })
        .unwrap()
    }
}

inventory::submit! {
    SourceDescription::new::<ConsulSourceConfig>("consul")
}

#[async_trait::async_trait]
#[typetag::serde(name = "consul")]
impl SourceConfig for ConsulSourceConfig {
    async fn build(&self, ctx: SourceContext) -> crate::Result<Source> {
        let proxy = ctx.proxy.clone();
        let tls = MaybeTlsSettings::from_config(&self.tls, false)?;
        let interval = tokio::time::interval(self.interval.into());
        let mut ticker = IntervalStream::new(interval).take_until(ctx.shutdown);
        let http_client = HttpClient::new(tls, &proxy)?;
        let health_summary = self.health_summary;
        let clients = self
            .endpoints
            .iter()
            .map(|endpoint| Client::new(endpoint.to_string(), http_client.clone()))
            .collect::<Vec<_>>();

        let mut output = ctx.out.sink_map_err(|err| {
            error!(
                message = "Error sending consul metrics",
                %err
            )
        });

        Ok(Box::pin(async move {
            while ticker.next().await.is_some() {
                let metrics = futures::future::join_all(
                    clients.iter().map(|cli| gather(cli, health_summary)),
                )
                .await;

                let mut stream = futures::stream::iter(metrics)
                    .map(futures::stream::iter)
                    .flatten()
                    .map(Event::Metric)
                    .map(Ok);

                output.send_all(&mut stream).await?
            }

            Ok(())
        }))
    }

    fn output_type(&self) -> DataType {
        DataType::Metric
    }

    fn source_type(&self) -> &'static str {
        "consul"
    }
}

async fn gather(client: &Client, health_summary: bool) -> Vec<Metric> {
    let start = Instant::now();
    let mut metrics = match tokio::try_join!(
        collect_peers_metric(client),
        collect_leader_metric(client),
        collect_nodes_metric(client),
        collect_members_metric(client),
        collect_services_metric(client, health_summary),
        collect_health_state_metric(client),
    ) {
        Ok((m1, m2, m3, m4, m5, m6)) => {
            let mut metrics =
                Vec::with_capacity(m1.len() + m2.len() + m3.len() + m4.len() + m5.len() + m6.len());
            metrics.extend(m1);
            metrics.extend(m2);
            metrics.extend(m3);
            metrics.extend(m4);
            metrics.extend(m5);
            metrics.extend(m6);

            metrics
        }
        Err(err) => {
            warn!(message = "Collect consul metrics failed", ?err);

            vec![Metric::gauge(
                "consul_up",
                "Was the last query of Consul successful",
                0,
            )]
        }
    };

    let elapsed = start.elapsed().as_secs_f64();
    metrics.push(Metric::gauge(
        "consule_scrape_duration_seconds",
        "",
        elapsed,
    ));

    metrics.iter_mut().for_each(|m| {
        m.tags
            .insert("instance".to_string(), client.endpoint.clone());
    });

    metrics
}

async fn collect_peers_metric(client: &Client) -> Result<Vec<Metric>, ConsulError> {
    let peers = client.peers().await?;

    Ok(vec![Metric::gauge(
        "consul_raft_peers",
        "How many peers (servers) are in the Raft cluster",
        peers.len(),
    )])
}

async fn collect_leader_metric(client: &Client) -> Result<Vec<Metric>, ConsulError> {
    let leader = client.leader().await? != "";

    Ok(vec![Metric::gauge(
        "consul_raft_leader",
        "Does Raft cluster have a leader (according to this node)",
        leader,
    )])
}

async fn collect_nodes_metric(client: &Client) -> Result<Vec<Metric>, ConsulError> {
    let nodes = client.nodes(None).await?;

    Ok(vec![Metric::gauge(
        "consul_serf_lan_members",
        "How many members are in the cluster",
        nodes.len(),
    )])
}

async fn collect_members_metric(client: &Client) -> Result<Vec<Metric>, ConsulError> {
    let members = client.members(false).await?;
    let mut metrics = Vec::with_capacity(members.len());

    for member in &members {
        metrics.push(Metric::gauge_with_tags(
            "consul_serf_lan_member_status",
            "Status of member in the cluster. 1=Alive, 2=Leaving, 3=Left, 4=Failed",
            member.status,
            tags!(
                "member" => &member.name
            ),
        ));
    }

    Ok(metrics)
}

async fn collect_services_metric(
    client: &Client,
    health_summary: bool,
) -> Result<Vec<Metric>, ConsulError> {
    let services = client.services(None).await?;

    let mut metrics = vec![Metric::gauge(
        "consul_catalog_services",
        "How many services are in the cluster",
        services.len(),
    )];

    if health_summary {
        futures::future::try_join_all(services.iter().map(|(name, _)| async move {
            let entries = match client.service(name, "", None).await {
                Ok(entries) => entries,
                Err(err) => {
                    warn!(
                        message = "Fetch service status failed",
                        service = ?name.to_owned(),
                        ?err
                    );

                    return Err(err);
                }
            };

            let mut used = HashSet::new();
            let mut metrics = vec![];

            for entry in &entries {
                // We have a Node, a Service, and one or more Checks. Our service-node
                // combo is passing if all checks have a `status` of "passing".
                let all_passing = entry.checks.iter().all(|hc| hc.status == "passing");

                metrics.push(Metric::gauge_with_tags(
                    "consul_catalog_service_node_healthy",
                    "Is this service healthy on this node",
                    all_passing,
                    tags!(
                        "service_id" => &entry.service.id,
                        "node" => &entry.node.node,
                        "service_name" => &entry.service.service
                    ),
                ));

                used.clear();
                for tag in &entry.service.tags {
                    if used.contains(tag) {
                        continue;
                    } else {
                        used.insert(tag);
                    }

                    metrics.push(Metric::gauge_with_tags(
                        "consul_service_tag",
                        "Tags of a service",
                        1,
                        tags!(
                            "service_id" => &entry.service.id,
                            "node" => &entry.node.node,
                            "tag" => tag
                        ),
                    ))
                }
            }

            Ok(metrics)
        }))
        .await?
        .iter()
        .for_each(|ms| metrics.extend_from_slice(ms));
    }

    Ok(metrics)
}

async fn collect_health_state_metric(client: &Client) -> Result<Vec<Metric>, ConsulError> {
    let health_state = client.health_state(None).await?;
    let mut metrics = vec![];
    let status_list = ["passing", "warning", "critical", "maintenance"];

    for hc in &health_state {
        if hc.service_id == "" {
            for status in status_list {
                metrics.push(Metric::gauge_with_tags(
                    "consul_health_node_status",
                    "Status of health checks associated with a node",
                    status == hc.status.as_str(),
                    tags!(
                        "check" => &hc.check_id,
                        "node" => &hc.node,
                        "status" => status
                    ),
                ));
            }
        } else {
            for status in status_list {
                metrics.push(Metric::gauge_with_tags(
                    "consul_health_service_status",
                    "Status of health checks associated with a service",
                    status == hc.status.as_str(),
                    tags!(
                        "check" => &hc.check_id,
                        "node" => &hc.node,
                        "service_id" => &hc.service_id,
                        "service_name" => &hc.service_name,
                        "status" => status,
                    ),
                ))
            }

            metrics.push(Metric::gauge_with_tags(
                "consul_service_checks",
                "Link the service id and check name if available",
                1,
                tags!(
                    "service_id" => &hc.service_id,
                    "service_name" => &hc.service_name,
                    "check_id" => &hc.check_id,
                    "check_name" => &hc.name,
                    "node" => &hc.node
                ),
            ))
        }
    }

    Ok(metrics)
}
