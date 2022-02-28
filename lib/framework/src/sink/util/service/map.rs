use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::{Layer, Service};

pub struct MapLayer<R1, R2> {
    f: Arc<dyn Fn(R1) -> R2 + Send + Sync + 'static>,
}

impl<R1, R2> MapLayer<R1, R2> {
    pub fn new(f: Arc<dyn Fn(R1) -> R2 + Send + Sync + 'static>) -> Self {
        Self { f }
    }
}

impl<S, R1, R2> Layer<S> for MapLayer<R1, R2>
where
    S: Service<R2>,
{
    type Service = Map<S, R1, R2>;

    fn layer(&self, inner: S) -> Self::Service {
        Map {
            f: Arc::clone(&self.f),
            inner,
        }
    }
}

pub struct Map<S, R1, R2> {
    f: Arc<dyn Fn(R1) -> R2 + Send + Sync + 'static>,
    pub(crate) inner: S,
}

impl<S, R1, R2> Service<R1> for Map<S, R1, R2>
where
    S: Service<R2>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: R1) -> Self::Future {
        let req = (self.f)(req);
        self.inner.call(req)
    }
}

impl<S, R1, R2> Clone for Map<S, R1, R2>
where
    S: Clone,
{
    fn clone(&self) -> Self {
        Self {
            f: Arc::clone(&self.f),
            inner: self.inner.clone(),
        }
    }
}

impl<S, R1, R2> Debug for Map<S, R1, R2>
where
    S: Debug,
{
    fn fmt(&self, fmt: &mut Formatter<'_>) -> std::fmt::Result {
        fmt.debug_struct("Map").field("inner", &self.inner).finish()
    }
}
