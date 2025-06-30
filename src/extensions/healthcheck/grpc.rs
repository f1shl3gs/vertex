//! Kubernetes now has built-in gRPC health probes starting in v1.23
//!
//! https://kubernetes.io/docs/tasks/configure-pod-container/configure-liveness-readiness-startup-probes/#define-a-grpc-liveness-probe

use std::net::SocketAddr;
use std::sync::atomic::Ordering;

use framework::ShutdownSignal;
use futures::FutureExt;
use tonic::transport::Error;
use tonic::{Request, Response, Status};
use tonic_health::pb::health_server::{Health, HealthServer};
use tonic_health::pb::{HealthCheckRequest, HealthCheckResponse};
use tonic_health::server::WatchStream;

struct HealthService;

#[async_trait::async_trait]
impl Health for HealthService {
    async fn check(
        &self,
        _req: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        let readiness = super::READINESS.load(Ordering::Acquire);

        Ok(Response::new(HealthCheckResponse {
            status: if readiness {
                // ServingStatus::Serving
                1
            } else {
                // ServingStatus::NotServing
                2
            },
        }))
    }

    type WatchStream = WatchStream;

    async fn watch(
        &self,
        _req: Request<HealthCheckRequest>,
    ) -> Result<Response<Self::WatchStream>, Status> {
        // Kubernetes does not implement watch
        //
        // https://github.com/kubernetes/kubernetes/blob/master/pkg/probe/grpc/grpc.go

        Err(Status::unimplemented("Not yet implemented"))
    }
}

pub async fn serve(addr: SocketAddr, shutdown: ShutdownSignal) -> Result<(), Error> {
    let service = HealthServer::new(HealthService);

    tonic::transport::Server::builder()
        .add_service(service)
        .serve_with_shutdown(addr, shutdown.map(|_| ()))
        .await
}
