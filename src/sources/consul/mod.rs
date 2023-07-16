mod client;
#[cfg(all(test, feature = "integration-tests-consul"))]
mod integration_test;

use std::collections::HashSet;
use std::time::{Duration, Instant};

use chrono::Utc;
use configurable::configurable_component;
use event::{tags, Metric};
use framework::config::{
    default_interval, default_true, DataType, Output, SourceConfig, SourceContext,
};
use framework::http::HttpClient;
use framework::tls::TlsConfig;
use framework::Source;

use crate::sources::consul::client::{Client, ConsulError, QueryOptions};

#[configurable_component(source, name = "consul")]
#[derive(Debug)]
#[serde(deny_unknown_fields)]
struct ConsulSourceConfig {
    /// HTTP/HTTPS endpoint to Consul server.
    #[configurable(required, format = "uri", example = "http://localhost:8500")]
    endpoints: Vec<String>,

    /// Duration between each scrape.
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,

    #[serde(default)]
    tls: Option<TlsConfig>,

    #[serde(default = "default_true")]
    health_summary: bool,

    #[serde(default)]
    query_options: Option<QueryOptions>,
}

#[async_trait::async_trait]
#[typetag::serde(name = "consul")]
impl SourceConfig for ConsulSourceConfig {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let SourceContext {
            mut shutdown,
            mut output,
            proxy,
            ..
        } = cx;
        let mut ticker = tokio::time::interval(self.interval);
        let http_client = HttpClient::new(&self.tls, &proxy)?;
        let health_summary = self.health_summary;
        let opts = self.query_options.clone();

        let clients = self
            .endpoints
            .iter()
            .map(|endpoint| Client::new(endpoint.to_string(), http_client.clone()))
            .collect::<Vec<_>>();

        Ok(Box::pin(async move {
            loop {
                tokio::select! {
                    biased;

                    _ = &mut shutdown => break,
                    _ = ticker.tick() => {}
                }

                let results = futures::future::join_all(
                    clients.iter().map(|cli| gather(cli, health_summary, &opts)),
                )
                .await;

                let now = Utc::now();
                let metrics = results
                    .into_iter()
                    .flatten()
                    .map(|mut m| {
                        m.timestamp = Some(now);
                        m
                    })
                    .collect::<Vec<_>>();

                if let Err(err) = output.send(metrics).await {
                    error!(
                        message = "Error sending consul metrics",
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
}

async fn gather(client: &Client, health_summary: bool, opts: &Option<QueryOptions>) -> Vec<Metric> {
    let start = Instant::now();
    let mut metrics = match tokio::try_join!(
        collect_peers_metric(client),
        collect_leader_metric(client),
        collect_nodes_metric(client, opts),
        collect_members_metric(client),
        collect_services_metric(client, health_summary, opts),
        collect_health_state_metric(client, opts),
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
            metrics.push(Metric::gauge(
                "consul_up",
                "Was the last query of Consul successful",
                1,
            ));

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
    metrics.push(Metric::gauge("consul_scrape_duration_seconds", "", elapsed));

    metrics.iter_mut().for_each(|m| {
        m.insert_tag("instance", &client.endpoint);
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
    let leader = client.leader().await?;

    Ok(vec![Metric::gauge(
        "consul_raft_leader",
        "Does Raft cluster have a leader (according to this node)",
        !leader.is_empty(),
    )])
}

async fn collect_nodes_metric(
    client: &Client,
    opts: &Option<QueryOptions>,
) -> Result<Vec<Metric>, ConsulError> {
    let nodes = client.nodes(opts).await?;

    Ok(vec![Metric::gauge(
        "consul_serf_lan_members",
        "How many members are in the cluster",
        nodes.len(),
    )])
}

async fn collect_members_metric(client: &Client) -> Result<Vec<Metric>, ConsulError> {
    let members = client.members().await?;
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
    opts: &Option<QueryOptions>,
) -> Result<Vec<Metric>, ConsulError> {
    let services = client.services(opts).await?;

    let mut metrics = vec![Metric::gauge(
        "consul_catalog_services",
        "How many services are in the cluster",
        services.len(),
    )];

    if health_summary {
        futures::future::try_join_all(services.keys().map(|name| async move {
            let entries = match client.service(name, opts).await {
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

async fn collect_health_state_metric(
    client: &Client,
    opts: &Option<QueryOptions>,
) -> Result<Vec<Metric>, ConsulError> {
    let health_state = client.health_state(opts).await?;
    let mut metrics = vec![];
    let status_list = ["passing", "warning", "critical", "maintenance"];

    for hc in &health_state {
        if hc.service_id.is_empty() {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<ConsulSourceConfig>()
    }
}
