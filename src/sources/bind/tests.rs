use std::future::Future;
use std::time::Duration;

use framework::config::ProxyConfig;
use framework::http::HttpClient;
use http::{Method, Request, Response};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Server};
use testify::{http::file_send, http::not_found, pick_unused_local_port};

use super::{client, statistics_to_metrics, Config};

#[test]
fn generate_config() {
    crate::testing::test_generate_config::<Config>()
}

async fn v3_handle(req: Request<Body>) -> hyper::Result<Response<Body>> {
    info!(
        message = "serve http request",
        path = req.uri().path(),
        handler = "v3"
    );

    if req.method() != Method::GET {
        return Ok(not_found());
    }

    let path = req.uri().path();
    for available in [
        "/xml/v3/server",
        "/xml/v3/status",
        "/xml/v3/tasks",
        "/xml/v3/zones",
    ] {
        if available == path {
            return file_send(available.replace("/xml", "tests/fixtures/bind").as_str()).await;
        }
    }

    Ok(not_found())
}

async fn v2_handle(req: Request<Body>) -> hyper::Result<Response<Body>> {
    info!(
        message = "serve http request",
        path = req.uri().path(),
        handler = "v2",
    );

    if req.method() != Method::GET {
        return Ok(not_found());
    }

    if req.uri().path() != "/" {
        Ok(not_found())
    } else {
        file_send("tests/fixtures/bind/v2.xml").await
    }
}

async fn start_server<H, S>(handle: H) -> String
where
    H: FnMut(Request<Body>) -> S + Copy + Send + Sync + 'static,
    S: Future<Output = hyper::Result<Response<Body>>> + Send + 'static,
{
    let port = pick_unused_local_port();
    let endpoint = format!("127.0.0.1:{}", port);
    let service =
        make_service_fn(move |_conn| async move { Ok::<_, hyper::Error>(service_fn(handle)) });
    let addr = endpoint.parse().unwrap();
    let server = Server::bind(&addr).serve(service);

    tokio::spawn(async move {
        if let Err(err) = server.await {
            error!(message = "server error", ?err);
        }
    });

    // sleep 1s to wait for the http server
    tokio::time::sleep(Duration::from_secs(1)).await;

    format!("http://{}", endpoint)
}

fn assert_statistics(s: client::Statistics, want: Vec<&str>) {
    #[allow(clippy::needless_collect)]
    let got = statistics_to_metrics(s)
        .into_iter()
        .map(|m| m.to_string())
        .flat_map(|s| s.lines().map(|s| s.to_string()).collect::<Vec<_>>())
        .collect::<Vec<_>>();

    for want in want {
        assert!(got.contains(&want.to_string()), "want {}", want)
    }
}

#[tokio::test]
async fn v2_client() {
    let endpoint = start_server(v2_handle).await;
    let http_client = HttpClient::new(&None, &ProxyConfig::default()).unwrap();
    let client = super::client::Client::new(endpoint, http_client);

    let s = client.stats().await.unwrap();

    assert_statistics(
        s,
        vec![
            // task
            "bind_tasks_running 8",
            "bind_worker_threads 16",
            // server
            "bind_boot_time_seconds 1626325868",
            r#"bind_incoming_queries_total{type="A"} 128417"#,
            r#"bind_incoming_requests_total{opcode="QUERY"} 37634"#,
            r#"bind_responses_total{result="Success"} 29313"#,
            "bind_query_duplicates_total 216",
            r#"bind_query_errors_total{error="Dropped"} 237"#,
            r#"bind_query_errors_total{error="Failure"} 2950"#,
            "bind_query_recursions_total 60946",
            "bind_zone_transfer_rejected_total 3",
            "bind_zone_transfer_success_total 25",
            "bind_zone_transfer_failure_total 1",
            "bind_recursive_clients 76",
            // view
            r#"bind_resolver_cache_rrsets{type="A",view="_default"} 34324"#,
            r#"bind_resolver_queries_total{type="CNAME",view="_default"} 28"#,
            r#"bind_resolver_response_errors_total{error="FORMERR",view="_bind"} 0"#,
            r#"bind_resolver_response_errors_total{error="FORMERR",view="_default"} 42906"#,
            r#"bind_resolver_response_errors_total{error="NXDOMAIN",view="_bind"} 0"#,
            r#"bind_resolver_response_errors_total{error="NXDOMAIN",view="_default"} 16707"#,
            r#"bind_resolver_response_errors_total{error="OtherError",view="_bind"} 0"#,
            r#"bind_resolver_response_errors_total{error="OtherError",view="_default"} 20660"#,
            r#"bind_resolver_response_errors_total{error="SERVFAIL",view="_bind"} 0"#,
            r#"bind_resolver_response_errors_total{error="SERVFAIL",view="_default"} 7596"#,
            r#"bind_resolver_response_lame_total{view="_default"} 9108"#,
            r#"bind_resolver_query_duration_seconds_bucket{le="0.01",view="_default"} 38334"#,
            r#"bind_resolver_query_duration_seconds_bucket{le="0.1",view="_default"} 113122"#,
            r#"bind_resolver_query_duration_seconds_bucket{le="0.5",view="_default"} 182658"#,
            r#"bind_resolver_query_duration_seconds_bucket{le="0.8",view="_default"} 187375"#,
            r#"bind_resolver_query_duration_seconds_bucket{le="1.6",view="_default"} 188409"#,
            r#"bind_resolver_query_duration_seconds_bucket{le="+Inf",view="_default"} 227755"#,
            r#"bind_zone_serial{view="_default",zone_name="TEST_ZONE"} 123"#,
        ],
    )
}

#[tokio::test]
async fn v3_client() {
    let endpoint = start_server(v3_handle).await;
    let http_client = HttpClient::new(&None, &ProxyConfig::default()).unwrap();
    let client = super::client::Client::new(endpoint, http_client);

    let s = client.stats().await.unwrap();
    assert_statistics(
        s,
        vec![
            // server
            r#"bind_config_time_seconds 1626325868"#,
            r#"bind_response_rcodes_total{rcode="NOERROR"} 989812"#,
            r#"bind_response_rcodes_total{rcode="NXDOMAIN"} 33958"#,
            // view
            r#"bind_resolver_response_errors_total{error="REFUSED",view="_bind"} 17"#,
            r#"bind_resolver_response_errors_total{error="REFUSED",view="_default"} 5798"#,
            // task
            r#"bind_tasks_running 8"#,
            r#"bind_worker_threads 16"#,
        ],
    )
}
