use std::sync::Arc;

use configurable::Configurable;
use http::HeaderValue;
use http::header::InvalidHeaderValue;
use serde::{Deserialize, Serialize};
use tonic::service::Interceptor;
use tonic::{Request, Status};

#[derive(Configurable, Debug, Deserialize, Serialize)]
pub struct Auth {
    /// The authentication username.
    pub username: String,

    /// The authentication password.
    pub password: String,
}

struct Inner {
    username: HeaderValue,
    password: HeaderValue,
}

#[derive(Clone)]
pub struct AuthInterceptor(Option<Arc<Inner>>);

impl AuthInterceptor {
    pub fn new(auth: Option<&Auth>) -> Result<Self, InvalidHeaderValue> {
        match auth {
            Some(auth) => {
                let inner = Inner {
                    username: HeaderValue::from_str(&auth.username)?,
                    password: HeaderValue::from_str(&auth.password)?,
                };

                Ok(AuthInterceptor(Some(Arc::new(inner))))
            }
            None => Ok(AuthInterceptor(None)),
        }
    }
}

impl Interceptor for AuthInterceptor {
    fn call(&mut self, mut req: Request<()>) -> Result<Request<()>, Status> {
        let Some(auth) = &self.0 else {
            return Ok(req);
        };

        let metadata = req.metadata_mut().as_mut();
        metadata.insert("username", auth.username.clone());
        metadata.insert("password", auth.password.clone());

        Ok(req)
    }
}
