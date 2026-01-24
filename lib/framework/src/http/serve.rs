use std::net::SocketAddr;

use futures::future::BoxFuture;
use http::{Request, Response};
use hyper::body::{Body, Incoming};
use hyper::service::Service;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder;

use crate::tls::MaybeTlsListener;
use crate::{Error, ShutdownSignal};

/// Serve the service with the supplied listener.
pub fn serve<R, S>(listener: MaybeTlsListener, service: S) -> Serve<S>
where
    S: Service<R> + Clone + Send + 'static,
    S::Future: Send,
{
    Serve { listener, service }
}

pub struct Serve<S> {
    listener: MaybeTlsListener,
    service: S,
}

impl<S> Serve<S> {
    /// Prepares a server to handle graceful shutdown when the provided ShutdownSignal future
    /// completes.
    pub fn with_graceful_shutdown(self, shutdown: ShutdownSignal) -> WithGracefulShutdown<S> {
        WithGracefulShutdown {
            listener: self.listener,
            service: self.service,
            shutdown,
        }
    }
}

/// Serve future with graceful shutdown enabled.
pub struct WithGracefulShutdown<S> {
    listener: MaybeTlsListener,
    shutdown: ShutdownSignal,
    service: S,
}

impl<S, B> IntoFuture for WithGracefulShutdown<S>
where
    B: Body + Send + 'static,
    B::Data: Send,
    B::Error: Into<Error>,
    S: Service<Request<Incoming>, Response = Response<B>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Into<Error>,
{
    type Output = Result<(), Error>;
    type IntoFuture = BoxFuture<'static, Result<(), Error>>;

    fn into_future(self) -> Self::IntoFuture {
        let WithGracefulShutdown {
            mut listener,
            service,
            mut shutdown,
        } = self;

        Box::pin(async move {
            loop {
                let (peer, conn) = tokio::select! {
                    _ = &mut shutdown => break,
                    result = listener.accept() => match result {
                        Ok(conn) => (conn.peer_addr(), TokioIo::new(conn)),
                        Err(err) => {
                            error!(
                                message = "accept new connection failed",
                                %err
                            );

                            continue;
                        }
                    }
                };

                let mut shutdown = shutdown.clone();
                let service = ConnectInfo {
                    peer,
                    inner: service.clone(),
                };
                tokio::spawn(async move {
                    let builder = Builder::new(TokioExecutor::new());
                    let conn = builder.serve_connection_with_upgrades(conn, service);
                    tokio::pin!(conn);

                    loop {
                        tokio::select! {
                            result = conn.as_mut() => {
                                if let Err(err) = result {
                                    trace!(
                                        message = "failed to serve http connection",
                                        %peer,
                                        %err
                                    );
                                }

                                break
                            }
                            _ = &mut shutdown => {
                                conn.as_mut().graceful_shutdown();
                            }
                        }
                    }
                });
            }

            Ok(())
        })
    }
}

struct ConnectInfo<S> {
    peer: SocketAddr,
    inner: S,
}

impl<B, S> Service<Request<B>> for ConnectInfo<S>
where
    S: Service<Request<B>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn call(&self, mut req: Request<B>) -> Self::Future {
        req.extensions_mut().insert(self.peer);
        self.inner.call(req)
    }
}
