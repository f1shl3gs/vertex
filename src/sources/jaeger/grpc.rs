use std::net::SocketAddr;

use async_trait::async_trait;
use configurable::Configurable;
use framework::{Pipeline, ShutdownSignal};
use futures_util::FutureExt;
use jaeger::proto::collector_service_server::CollectorServiceServer;
use jaeger::proto::{CollectorService, PostSpansRequest, PostSpansResponse};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tonic::{transport::Server, Request, Response, Status};

fn default_grpc_endpoint() -> SocketAddr {
    SocketAddr::new([0, 0, 0, 0].into(), 14250)
}

/// In a typical Jaeger deployment, Agents receive spans from Clients and forward them to Collectors
///
/// See https://www.jaegertracing.io/docs/1.31/apis/#protobuf-via-grpc-stable
#[derive(Configurable, Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GrpcServerConfig {
    #[configurable(required)]
    #[serde(default = "default_grpc_endpoint")]
    pub endpoint: SocketAddr,
}

struct JaegerCollector {
    output: Mutex<Pipeline>,
}

#[async_trait]
impl CollectorService for JaegerCollector {
    async fn post_spans(
        &self,
        request: Request<PostSpansRequest>,
    ) -> Result<Response<PostSpansResponse>, Status> {
        let req = request.into_inner();
        if let Some(batch) = req.batch {
            let mut output = self.output.lock().await;

            if let Err(err) = output.send(batch).await {
                warn!(message = "Error sending trace", %err);
                return Err(Status::internal(err.to_string()));
            }
        }

        Ok(Response::new(PostSpansResponse {}))
    }
}

pub(super) async fn serve(
    config: GrpcServerConfig,
    shutdown: ShutdownSignal,
    output: Pipeline,
) -> crate::Result<()> {
    let service = CollectorServiceServer::new(JaegerCollector {
        output: Mutex::new(output),
    });

    Server::builder()
        .add_service(service)
        .serve_with_shutdown(config.endpoint, shutdown.map(|_| ()))
        .await
        .map_err(|err| {
            warn!(message = "Grpc server exit", %err);
            err.into()
        })
}
