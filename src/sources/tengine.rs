use std::io::BufRead;
use std::time::Duration;

use bytes::Buf;
use configurable::configurable_component;
use event::{Metric, tags};
use framework::config::default_interval;
use framework::config::{OutputType, SourceConfig, SourceContext};
use framework::http::HttpClient;
use framework::tls::TlsConfig;
use framework::{Pipeline, ShutdownSignal, Source};
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use http::Request;
use http_body_util::{BodyExt, Full};
use url::Url;

use crate::Error;

/// The source collect metrics from the [Tengine Web Server](http://tengine.taobao.org/)
/// via the [ngx_http_reqstat_module](https://tengine.taobao.org/document/http_reqstat.html) module.
#[configurable_component(source, name = "tengine")]
struct Config {
    /// targets to scrape
    targets: Vec<Url>,

    tls: Option<TlsConfig>,

    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,
}

#[async_trait::async_trait]
#[typetag::serde(name = "tengine")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let client = HttpClient::new(self.tls.as_ref(), &cx.proxy)?;

        Ok(Box::pin(run(
            client,
            self.targets.clone(),
            self.interval,
            cx.output,
            cx.shutdown,
        )))
    }

    fn outputs(&self) -> Vec<OutputType> {
        vec![OutputType::metric()]
    }
}

async fn run(
    client: HttpClient,
    targets: Vec<Url>,
    interval: Duration,
    mut output: Pipeline,
    mut shutdown: ShutdownSignal,
) -> Result<(), ()> {
    let mut ticker = tokio::time::interval(interval);

    loop {
        tokio::select! {
            _ = ticker.tick() => {},
            _ = &mut shutdown => break,
        }

        let mut tasks =
            FuturesUnordered::from_iter(targets.iter().map(|target| collect(&client, target)));

        loop {
            let metrics = tokio::select! {
                _ = &mut shutdown => return Ok(()),
                Some(result) = tasks.next() => match result {
                    Ok(metrics) => metrics,
                    Err(err) => {
                        warn!(message = "failed to collect metrics", %err);
                        break
                    }
                },
                else => break,
            };

            if let Err(_err) = output.send(metrics).await {
                return Ok(());
            }
        }
    }

    Ok(())
}

const METRIC_INFOS: [(&str, &str); 28] = [
    (
        "tengine_bytes_in",
        "total number of bytes received from client",
    ),
    ("tengine_bytes_out", "total number of bytes sent to client"),
    ("tengine_conn_total", "total number of accepted connections"),
    ("tengine_req_total", "total number of processed requests"),
    ("tengine_http_2xx", "total number of 2xx requests"),
    ("tengine_http_3xx", "total number of 3xx requests"),
    ("tengine_http_4xx", "total number of 4xx requests"),
    ("tengine_http_5xx", "total number of 5xx requests"),
    (
        "tengine_http_other_status",
        "total number of other requests",
    ),
    ("tengine_rt", "accumulation or rt"),
    (
        "tengine_ups_req",
        "total number of requests calling for upstream",
    ),
    ("tengine_ups_rt", "accumulation or upstream rt"),
    (
        "tengine_ups_tries",
        "total number of times calling for upstream",
    ),
    ("tengine_http_200", "total number of 200 requests"),
    ("tengine_http_206", "total number of 206 requests"),
    ("tengine_http_302", "total number of 302 requests"),
    ("tengine_http_304", "total number of 304 requests"),
    ("tengine_http_403", "total number of 403 requests"),
    ("tengine_http_404", "total number of 404 requests"),
    ("tengine_http_416", "total number of 416 requests"),
    ("tengine_http_499", "total number of 499 requests"),
    ("tengine_http_500", "total number of 500 requests"),
    ("tengine_http_502", "total number of 502 requests"),
    ("tengine_http_503", "total number of 503 requests"),
    ("tengine_http_504", "total number of 504 requests"),
    ("tengine_http_508", "total number of 508 requests"),
    (
        "tengine_http_other_detail_status",
        "total number of requests of other status codes* http_ups_4xx total number of requests of upstream 4xx",
    ),
    (
        "tengine_http_ups_5xx",
        "total number of requests of upstream 5xx",
    ),
];

async fn collect(client: &HttpClient, target: &Url) -> Result<Vec<Metric>, Error> {
    let req = Request::builder()
        .uri(target.as_str())
        .body(Full::default())?;

    let resp = client.send(req).await?;
    let (parts, incoming) = resp.into_parts();
    if !parts.status.is_success() {
        debug!(
            message = "request to tengine failed",
            target = target.as_str(),
            status = parts.status.as_u16()
        );

        return Err("invalid status".into());
    }

    let mut body = incoming.collect().await?.aggregate().reader();

    let mut metrics = Vec::new();
    let mut line = String::new();
    loop {
        line.clear();
        if body.read_line(&mut line)? == 0 {
            break;
        }

        let mut fields = line.split(',');
        let Some(host) = fields.next() else { continue };

        metrics.reserve(METRIC_INFOS.len());
        for (field, (name, desc)) in fields.zip(METRIC_INFOS) {
            let Ok(value) = field.parse::<u64>() else {
                return Err("malformed host stats line".into());
            };

            metrics.push(Metric::sum_with_tags(
                name,
                desc,
                value,
                tags!("host" => host),
            ))
        }
    }

    Ok(metrics)
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use framework::config::ProxyConfig;
    use http::Response;
    use hyper::body::Incoming;
    use hyper::server::conn::http1;
    use hyper::service::service_fn;
    use hyper_util::rt::TokioIo;
    use testify::wait::wait_for_tcp;
    use tokio::net::TcpListener;

    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>();
    }

    #[tokio::test]
    async fn e2e() {
        async fn handle(_req: Request<Incoming>) -> Result<Response<Full<Bytes>>, http::Error> {
            let body = "127.0.0.1,784,1511,2,2,1,0,1,0,0,0,0,0,0,1,0,0,0,0,1,0,0,0,0,0,0,0,0,0,0\n127.0.0.2,784,1511,2,2,1,0,1,0,0,0,0,0,0,1,0,0,0,0,1,0,0,0,0,0,0,0,0,0,0";

            Response::builder()
                .status(200)
                .body(Full::new(Bytes::from_static(body.as_ref())))
        }

        let addr = testify::next_addr();
        let listener = TcpListener::bind(addr).await.unwrap();

        tokio::spawn(async move {
            loop {
                let (stream, _) = listener.accept().await.unwrap();
                let io = TokioIo::new(stream);

                tokio::task::spawn(async move {
                    if let Err(err) = http1::Builder::new()
                        .serve_connection(io, service_fn(handle))
                        .await
                    {
                        println!("Error serving connection: {:?}", err);
                    }
                });
            }
        });

        wait_for_tcp(addr).await;

        let client = HttpClient::new(None, &ProxyConfig::default()).unwrap();
        let target = Url::parse(&format!("http://{addr}")).unwrap();

        let metrics = collect(&client, &target).await.unwrap();
        assert_eq!(metrics.len(), 2 * 28);
    }
}
