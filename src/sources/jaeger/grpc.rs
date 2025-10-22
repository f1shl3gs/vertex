use std::net::SocketAddr;

use configurable::Configurable;
use event::{AddBatchNotifier, BatchNotifier, BatchStatus, Events};
use framework::{Pipeline, ShutdownSignal};
use futures::FutureExt;
use jaeger::proto::collector_service_server::CollectorServiceServer;
use jaeger::proto::{CollectorService, PostSpansRequest, PostSpansResponse};
use serde::{Deserialize, Serialize};
use tonic::{Request, Response, Status, transport::Server};

fn default_listen() -> SocketAddr {
    SocketAddr::from(([0, 0, 0, 0], 14250))
}

/// In a typical Jaeger deployment, Agents receive spans from Clients and forward them to Collectors
///
/// See https://www.jaegertracing.io/docs/1.31/apis/#protobuf-via-grpc-stable
#[derive(Configurable, Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GrpcServerConfig {
    #[serde(default = "default_listen")]
    pub listen: SocketAddr,
}

struct JaegerCollector {
    output: Pipeline,

    acknowledgements: bool,
}

#[async_trait::async_trait]
impl CollectorService for JaegerCollector {
    async fn post_spans(
        &self,
        request: Request<PostSpansRequest>,
    ) -> Result<Response<PostSpansResponse>, Status> {
        let req = request.into_inner();
        if let Some(batch) = req.batch {
            let mut events: Events = batch.into();

            let (batch, receiver) = BatchNotifier::maybe_new_with_receiver(self.acknowledgements);
            if let Some(batch) = batch {
                events.add_batch_notifier(batch);
            }

            if let Err(err) = self.output.clone().send(events).await {
                warn!(message = "Error sending trace", %err);
                return Err(Status::internal(err.to_string()));
            }

            return if let Some(receiver) = receiver {
                match receiver.await {
                    BatchStatus::Delivered => Ok(Response::new(PostSpansResponse {})),
                    BatchStatus::Errored => Err(Status::data_loss("Jaeger errored")),
                    BatchStatus::Failed => Err(Status::unavailable("traces deliver failed")),
                }
            } else {
                Ok(Response::new(PostSpansResponse {}))
            };
        }

        Ok(Response::new(PostSpansResponse {}))
    }
}

pub(super) async fn serve(
    config: GrpcServerConfig,
    shutdown: ShutdownSignal,
    output: Pipeline,
    acknowledgements: bool,
) -> crate::Result<()> {
    let service = CollectorServiceServer::new(JaegerCollector {
        output,
        acknowledgements,
    });

    Server::builder()
        .add_service(service)
        .serve_with_shutdown(config.listen, shutdown.map(|_| ()))
        .await
        .map_err(|err| {
            warn!(message = "Grpc server exit", %err);
            err.into()
        })
}
