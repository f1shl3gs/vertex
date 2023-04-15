use async_trait::async_trait;
use event::Event;
use framework::{Pipeline, ShutdownSignal};
use futures_util::FutureExt;
use jaeger::proto::collector_service_server::CollectorServiceServer;
use jaeger::proto::{CollectorService, PostSpansRequest, PostSpansResponse};
use tokio::sync::Mutex;
use tonic::{transport::Server, Request, Response, Status};

use super::GrpcServerConfig;

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

            let event = Event::from(batch);
            if let Err(err) = output.send(event).await {
                let err = format!("{:?}", err);
                warn!(message = "Error sending trace", %err);
                return Err(Status::internal(err));
            }
        }

        Ok(Response::new(PostSpansResponse {}))
    }
}

pub(super) async fn serve(
    config: GrpcServerConfig,
    shutdown: ShutdownSignal,
    output: Pipeline,
) -> framework::Result<()> {
    let service = CollectorServiceServer::new(JaegerCollector {
        output: Mutex::new(output),
    });

    Server::builder()
        .add_service(service)
        .serve_with_shutdown(config.endpoint, shutdown.map(|_| ()))
        .await
        .map_err(|err| {
            warn!(message = "Grpc server exit", ?err);
            err.into()
        })
}
