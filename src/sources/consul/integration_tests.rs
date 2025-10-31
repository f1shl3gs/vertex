use std::net::SocketAddr;
use std::time::Duration;

use event::{Metric, MetricValue, tags};
use framework::config::ProxyConfig;
use framework::http::HttpClient;
use http::StatusCode;
use http_body_util::Full;
use serde::{Deserialize, Serialize};
use testify::container::Container;
use testify::next_addr;

use super::{Client, ConsulError, gather};
use crate::testing::trace_init;

#[tokio::test]
async fn client() {
    trace_init();

    let service_addr = next_addr();

    Container::new("consul", "1.15.4")
        .with_tcp(8500, service_addr.port())
        .run(async move {
            // tokio::time::sleep(Duration::from_secs(200)).await;

            let client = HttpClient::new(None, &ProxyConfig::default()).unwrap();
            let client = Client::new(format!("http://{service_addr}"), client);

            let peers = client.peers().await.unwrap();
            assert_eq!(peers.len(), 1);
            assert_eq!(peers[0], "127.0.0.1:8300".to_string());

            tokio::time::sleep(Duration::from_secs(5)).await;

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
        })
        .await;
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct ServiceCheck {
    #[serde(rename = "CheckID")]
    pub check_id: String,
    pub name: String,
    pub tcp: String,
    #[serde(with = "humanize::duration::serde")]
    pub timeout: Duration,
    #[serde(with = "humanize::duration::serde")]
    pub interval: Duration,
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
    let path = format!("{endpoint}/v1/agent/service/register");
    let body = serde_json::to_vec(svc).unwrap();
    let req = http::Request::put(path)
        .body(Full::new(body.into()))
        .unwrap();

    match client.send(req).await {
        Ok(resp) => {
            let (parts, _incoming) = resp.into_parts();
            match parts.status {
                StatusCode::OK => Ok(()),
                status => Err(ConsulError::UnexpectedStatusCode(status.as_u16())),
            }
        }
        Err(err) => Err(ConsulError::HttpErr(err)),
    }
}

const NODE_ID: &str = "adf4238a-882b-9ddc-4a9d-5b6758e4159e";
const NODE_NAME: &str = "6decb76944f6";

async fn run(services: &[ServiceRegistration]) -> (SocketAddr, Vec<Metric>) {
    trace_init();

    let service_addr = next_addr();
    let metrics = Container::new("consul", "1.15.4")
        .args([
            "agent",
            "-data-dir=/consul/data",
            "-config-dir=/consul/config",
            "-dev",
            "-node-id",
            NODE_ID,
            "-node",
            NODE_NAME,
            "-client",
            "0.0.0.0",
        ])
        .with_tcp(8500, service_addr.port())
        .run(async move {
            // wait for raft stable
            tokio::time::sleep(Duration::from_secs(5)).await;

            let endpoint = format!("http://{service_addr}");
            let http_client = HttpClient::new(None, &ProxyConfig::default()).unwrap();
            for svc in services {
                register_service(&endpoint, &http_client, svc)
                    .await
                    .unwrap();
            }

            let client = Client::new(endpoint.clone(), http_client);
            gather(&client, true, &None).await
        })
        .await;

    (service_addr, metrics)
}

#[tokio::test]
async fn simple_collect() {
    let (instance, metrics) = run(&[]).await;

    for (name, mut tags, value) in vec![
        (
            "consul_catalog_service_node_healthy",
            tags!(
                "node" => NODE_NAME,
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
                "node" => NODE_NAME,
                "status" => "critical"
            ),
            MetricValue::gauge(0),
        ),
        (
            "consul_health_node_status",
            tags!(
                "check" => "serfHealth",
                "node" => NODE_NAME,
                "status" => "maintenance"
            ),
            MetricValue::gauge(0),
        ),
        (
            "consul_health_node_status",
            tags!(
                "check" => "serfHealth",
                "node" => NODE_NAME,
                "status" => "passing"
            ),
            MetricValue::gauge(1),
        ),
        (
            "consul_health_node_status",
            tags!(
                "check" => "serfHealth",
                "node" => NODE_NAME,
                "status" => "warning"
            ),
            MetricValue::gauge(0),
        ),
        ("consul_raft_leader", tags!(), MetricValue::gauge(1)),
        ("consul_raft_peers", tags!(), MetricValue::gauge(1)),
        (
            "consul_serf_lan_member_status",
            tags!(
                "member" => NODE_NAME,
            ),
            MetricValue::gauge(1),
        ),
        ("consul_serf_lan_members", tags!(), MetricValue::gauge(1)),
        ("consul_up", tags!(), MetricValue::gauge(1)),
    ] {
        tags.insert("instance".to_string(), format!("http://{instance}"));

        assert!(
            metrics
                .iter()
                .any(|m| m.name() == "consul_scrape_duration_seconds")
        );

        assert!(
            metrics
                .iter()
                .any(|m| m.name() == name && m.tags() == &tags && m.value == value),
            "want {name} {tags:?} {value:?}\n\n{metrics:#?}",
        );
    }
}

#[tokio::test]
async fn duplicate_tag_values() {
    let (instance, metrics) = run(&[ServiceRegistration {
        id: "foo".to_string(),
        name: "foo".to_string(),
        tags: vec!["tag1".to_string(), "tag2".to_string(), "tag1".to_string()],
        checks: vec![],
    }])
    .await;

    for (name, mut tags, value) in [
        (
            "consul_catalog_service_node_healthy",
            tags!(
                "node" => NODE_NAME,
                "service_id" => "consul",
                "service_name" => "consul"
            ),
            MetricValue::gauge(1),
        ),
        (
            "consul_catalog_service_node_healthy",
            tags!(
                "node" => NODE_NAME,
                "service_id" => "foo",
                "service_name" => "foo"
            ),
            MetricValue::gauge(1),
        ),
        ("consul_catalog_services", tags!(), MetricValue::gauge(2)),
        (
            "consul_service_tag",
            tags!(
                "node" => NODE_NAME,
                "service_id" => "foo",
                "tag" => "tag1",
            ),
            MetricValue::gauge(1),
        ),
        (
            "consul_service_tag",
            tags!(
                "node" => NODE_NAME,
                "service_id" => "foo",
                "tag" => "tag2",
            ),
            MetricValue::gauge(1),
        ),
    ] {
        tags.insert("instance".to_string(), format!("http://{instance}"));

        assert!(
            metrics
                .iter()
                .any(|m| m.name() == "consul_scrape_duration_seconds")
        );

        assert!(
            metrics
                .iter()
                .any(|m| m.name() == name && m.tags() == &tags && m.value == value),
            "want {name} {tags:?} {value:?}\n\n{metrics:#?}",
        );
    }
}

#[tokio::test]
async fn forward_slash_service_name() {
    let (instance, metrics) = run(&[
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
    ])
    .await;

    for (name, mut tags, value) in [
        (
            "consul_catalog_service_node_healthy",
            tags!(
                "node" => NODE_NAME,
                "service_id" => "bar",
                "service_name" => "bar",
            ),
            MetricValue::gauge(1),
        ),
        (
            "consul_catalog_service_node_healthy",
            tags!(
                "node" => NODE_NAME,
                "service_id" => "consul",
                "service_name" => "consul",
            ),
            MetricValue::gauge(1),
        ),
        ("consul_catalog_services", tags!(), MetricValue::gauge(3)),
    ] {
        tags.insert("instance".to_string(), format!("http://{instance}"));

        assert!(
            metrics
                .iter()
                .any(|m| m.name() == "consul_scrape_duration_seconds")
        );

        assert!(
            metrics
                .iter()
                .any(|m| m.name() == name && m.tags() == &tags && m.value == value),
            "want {name} {tags:?} {value:?}\n\n{metrics:#?}",
        );
    }
}

#[tokio::test]
async fn service_check_name() {
    let (instance, metrics) = run(&[ServiceRegistration {
        id: "special".to_string(),
        name: "special".to_string(),
        tags: vec![],
        checks: vec![ServiceCheck {
            check_id: "_nomad-check-special".to_string(),
            name: "friendly-name".to_string(),
            tcp: "localhost:8080".to_string(),
            timeout: Duration::from_secs(30),
            interval: Duration::from_secs(10),
        }],
    }])
    .await;

    let (name, mut tags, value) = (
        "consul_service_checks",
        tags!(
            "check_id" => "_nomad-check-special",
            "check_name" => "friendly-name",
            "node" => NODE_NAME,
            "service_id" => "special",
            "service_name" => "special"
        ),
        MetricValue::gauge(1),
    );
    tags.insert("instance".to_string(), format!("http://{instance}"));

    assert!(
        metrics
            .iter()
            .any(|m| m.name() == "consul_scrape_duration_seconds")
    );

    assert!(
        metrics
            .iter()
            .any(|m| m.name() == name && m.tags() == &tags && m.value == value),
        "want {name} {tags:?} {value:?}\n\n{metrics:#?}",
    );
}
