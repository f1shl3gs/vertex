use event::{tags, MetricValue};
use framework::config::ProxyConfig;
use framework::http::HttpClient;
use http::StatusCode;
use hyper::Body;
use serde::{Deserialize, Serialize};

use super::{gather, Client, ConsulError};
use crate::testing::{ContainerBuilder, WaitFor};

#[tokio::test]
async fn test_client() {
    let container = ContainerBuilder::new("consul:1.11.1")
        .port(8500)
        .run()
        .unwrap();
    container.wait(WaitFor::Stdout("Synced node info")).unwrap();
    let endpoint = container.get_host_port(8500).unwrap();
    let client = HttpClient::new(&None, &ProxyConfig::default()).unwrap();
    let client = Client::new(endpoint, client);

    let peers = client.peers().await.unwrap();
    assert_eq!(peers.len(), 1);
    assert_eq!(peers[0], "127.0.0.1:8300".to_string());

    let leader = client.leader().await.unwrap();
    assert_eq!(leader, "127.0.0.1:8300".to_string());

    let nodes = client.nodes(&None).await.unwrap();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].address, "127.0.0.1");

    let members = client.members().await.unwrap();
    assert_eq!(members.len(), 1);
    assert_eq!(members[0].status, 1.0);
    assert_eq!(members[0].addr, "127.0.0.1".to_string());

    let services = client.services(&None).await.unwrap();
    assert_eq!(services.len(), 1);
    assert_eq!(services.get("consul").unwrap().len(), 0);

    let entries = client.service("consul", &None).await.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].node.address, "127.0.0.1".to_string());
    assert_eq!(entries[0].service.service, "consul".to_string());

    let health_states = client.health_state(&None).await.unwrap();
    assert_eq!(health_states.len(), 1);
    assert_eq!(health_states[0].name, "Serf Health Status".to_string());
    assert_eq!(health_states[0].status, "passing".to_string());
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct ServiceCheck {
    #[serde(rename = "CheckID")]
    pub check_id: String,
    pub name: String,
    pub tcp: String,
    #[serde(with = "humanize::duration::serde")]
    pub timeout: std::time::Duration,
    #[serde(with = "humanize::duration::serde")]
    pub interval: std::time::Duration,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct ServiceRegistration {
    #[serde(rename = "ID")]
    pub id: String,
    pub name: String,
    pub tags: Vec<String>,
    pub checks: Vec<ServiceCheck>,
}

async fn register_service(
    endpoint: &str,
    client: &HttpClient,
    svc: &ServiceRegistration,
) -> Result<(), ConsulError> {
    let path = format!("{}/v1/agent/service/register", endpoint);
    let body = serde_json::to_vec(svc).unwrap();
    let req = http::Request::put(path).body(Body::from(body)).unwrap();

    match client.send(req).await {
        Ok(resp) => {
            let (parts, _body) = resp.into_parts();
            match parts.status {
                StatusCode::OK => Ok(()),
                status => Err(ConsulError::UnexpectedStatusCode(status.as_u16())),
            }
        }
        Err(err) => Err(ConsulError::HttpErr(err)),
    }
}

#[tokio::test]
async fn test_gather() {
    let node_id = "adf4238a-882b-9ddc-4a9d-5b6758e4159e";
    let node_name = "6decb76944f6";

    let tests = [
        (
            "simple collect",
            vec![],
            vec![
                (
                    "consul_catalog_service_node_healthy",
                    tags!(
                        "node" => node_name,
                        "service_id" => "consul",
                        "service_name" => "consul"
                    ),
                    MetricValue::Gauge(1.0),
                ),
                ("consul_catalog_services", tags!(), MetricValue::Gauge(1.0)),
                (
                    "consul_health_node_status",
                    tags!(
                        "check" => "serfHealth",
                        "node" => node_name,
                        "status" => "critical"
                    ),
                    MetricValue::gauge(0),
                ),
                (
                    "consul_health_node_status",
                    tags!(
                        "check" => "serfHealth",
                        "node" => node_name,
                        "status" => "maintenance"
                    ),
                    MetricValue::gauge(0),
                ),
                (
                    "consul_health_node_status",
                    tags!(
                        "check" => "serfHealth",
                        "node" => node_name,
                        "status" => "passing"
                    ),
                    MetricValue::gauge(1),
                ),
                (
                    "consul_health_node_status",
                    tags!(
                        "check" => "serfHealth",
                        "node" => node_name,
                        "status" => "warning"
                    ),
                    MetricValue::gauge(0),
                ),
                ("consul_raft_leader", tags!(), MetricValue::gauge(1)),
                ("consul_raft_peers", tags!(), MetricValue::gauge(1)),
                (
                    "consul_serf_lan_member_status",
                    tags!(
                        "member" => node_name,
                    ),
                    MetricValue::gauge(1),
                ),
                ("consul_serf_lan_members", tags!(), MetricValue::gauge(1)),
                ("consul_up", tags!(), MetricValue::gauge(1)),
            ],
        ),
        (
            "collect with duplicate tag values",
            vec![ServiceRegistration {
                id: "foo".to_string(),
                name: "foo".to_string(),
                tags: vec!["tag1".to_string(), "tag2".to_string(), "tag1".to_string()],
                checks: vec![],
            }],
            vec![
                (
                    "consul_catalog_service_node_healthy",
                    tags!(
                        "node" => node_name,
                        "service_id" => "consul",
                        "service_name" => "consul"
                    ),
                    MetricValue::gauge(1),
                ),
                (
                    "consul_catalog_service_node_healthy",
                    tags!(
                        "node" => node_name,
                        "service_id" => "foo",
                        "service_name" => "foo"
                    ),
                    MetricValue::gauge(1),
                ),
                ("consul_catalog_services", tags!(), MetricValue::gauge(2)),
                (
                    "consul_service_tag",
                    tags!(
                        "node" => node_name,
                        "service_id" => "foo",
                        "tag" => "tag1",
                    ),
                    MetricValue::gauge(1),
                ),
                (
                    "consul_service_tag",
                    tags!(
                        "node" => node_name,
                        "service_id" => "foo",
                        "tag" => "tag2",
                    ),
                    MetricValue::gauge(1),
                ),
            ],
        ),
        (
            "collect with forward slash service name",
            vec![
                ServiceRegistration {
                    id: "slashbar".to_string(),
                    name: "/bar".to_string(),
                    tags: vec![],
                    checks: vec![],
                },
                ServiceRegistration {
                    id: "bar".to_string(),
                    name: "bar".to_string(),
                    tags: vec![],
                    checks: vec![],
                },
            ],
            vec![
                (
                    "consul_catalog_service_node_healthy",
                    tags!(
                        "node" => node_name,
                        "service_id" => "bar",
                        "service_name" => "bar",
                    ),
                    MetricValue::gauge(1),
                ),
                (
                    "consul_catalog_service_node_healthy",
                    tags!(
                        "node" => node_name,
                        "service_id" => "consul",
                        "service_name" => "consul",
                    ),
                    MetricValue::gauge(1),
                ),
                ("consul_catalog_services", tags!(), MetricValue::gauge(3)),
            ],
        ),
        (
            "collect with service check name",
            vec![ServiceRegistration {
                id: "special".to_string(),
                name: "special".to_string(),
                tags: vec![],
                checks: vec![ServiceCheck {
                    check_id: "_nomad-check-special".to_string(),
                    name: "friendly-name".to_string(),
                    tcp: "localhost:8080".to_string(),
                    timeout: std::time::Duration::from_secs(30),
                    interval: std::time::Duration::from_secs(10),
                }],
            }],
            vec![(
                "consul_service_checks",
                tags!(
                    "check_id" => "_nomad-check-special",
                    "check_name" => "friendly-name",
                    "node" => node_name,
                    "service_id" => "special",
                    "service_name" => "special"
                ),
                MetricValue::gauge(1),
            )],
        ),
    ];

    for (test, services, wants) in tests {
        let container = ContainerBuilder::new("consul:1.11.1")
            .args([
                "agent",
                "-data-dir=/consul/data",
                "-config-dir=/consul/config",
                "-dev",
                "-node-id",
                node_id,
                "-node",
                node_name,
                "-client",
                "0.0.0.0",
            ])
            .port(8500)
            .run()
            .unwrap();
        container.wait(WaitFor::Stdout("Synced node info")).unwrap();

        let host_port = container.get_host_port(8500).unwrap();
        let endpoint = format!("http://{}", host_port);
        let http_client = HttpClient::new(&None, &ProxyConfig::default()).unwrap();
        for svc in &services {
            register_service(&endpoint, &http_client, svc)
                .await
                .unwrap();
        }

        let client = Client::new(endpoint.clone(), http_client);
        let metrics = gather(&client, true, &None).await;
        for (name, mut tags, value) in wants {
            tags.insert("instance".to_string(), endpoint.clone());

            assert!(metrics
                .iter()
                .any(|m| m.name() == "consul_scrape_duration_seconds"));

            assert!(
                metrics
                    .iter()
                    .any(|m| m.name() == name && m.tags() == &tags && m.value == value),
                "Case \"{}\" want {} {:?} {:?}\n{:#?}",
                test,
                name,
                tags,
                value,
                metrics
            );
        }
    }
}
