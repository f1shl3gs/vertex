use configurable::Configurable;
use headers::{Authorization, HeaderMapExt};
use http::header::AUTHORIZATION;
use http::request::Builder;
use http::{HeaderMap, Request};
use hyper::body::Body;
use serde::{Deserialize, Serialize};

/// The authentication strategy for http request/response
#[derive(Configurable, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case", tag = "strategy")]
pub enum Auth {
    /// Basic authentication.
    ///
    /// The username and password are concatenated and encoded via [base64][base64].
    ///
    /// [base64]: https://en.wikipedia.org/wiki/Base64
    Basic {
        /// The basic authentication username.
        user: String,

        /// The basic authentication password.
        password: String,
    },

    /// Bearer authentication.
    ///
    /// The bearer token value (OAuth2, JWT, etc) is passed as-is.
    Bearer {
        /// The bearer authentication token.
        token: String,
    },
}

impl Auth {
    pub fn basic(user: String, password: String) -> Self {
        Self::Basic { user, password }
    }

    pub fn apply<B>(&self, req: &mut Request<B>) {
        self.apply_headers_map(req.headers_mut())
    }

    pub fn authorizer(&self) -> Authorizer {
        match self {
            Auth::Basic { user, password } => {
                use base64::prelude::{BASE64_STANDARD, Engine as _};

                let token = BASE64_STANDARD.encode(format!("{user}:{password}"));

                Authorizer::Basic(format!("Basic {token}"))
            }
            Auth::Bearer { token } => Authorizer::Bearer(format!("Bearer {}", token)),
        }
    }

    pub fn apply_builder(&self, mut builder: Builder) -> Builder {
        if let Some(map) = builder.headers_mut() {
            self.apply_headers_map(map)
        }
        builder
    }

    pub fn apply_headers_map(&self, map: &mut HeaderMap) {
        match &self {
            Auth::Basic { user, password } => {
                let auth = Authorization::basic(user, password);
                map.typed_insert(auth);
            }
            Auth::Bearer { token } => match Authorization::bearer(token) {
                Ok(auth) => map.typed_insert(auth),
                Err(err) => error!(message = "Invalid bearer token", token = %token, %err),
            },
        }
    }
}

pub trait MaybeAuth: Sized {
    fn choose_one(&self, other: &Self) -> crate::Result<Self>;
}

impl MaybeAuth for Option<Auth> {
    fn choose_one(&self, other: &Self) -> crate::Result<Self> {
        if self.is_some() && other.is_some() {
            Err("Two authorization credentials was provided.".into())
        } else {
            Ok(self.clone().or_else(|| other.clone()))
        }
    }
}

#[derive(Clone, Debug)]
pub enum Authorizer {
    Basic(String),
    Bearer(String),
}

impl Authorizer {
    pub fn authorized<T: Body>(&self, req: &Request<T>) -> bool {
        let Some(value) = req.headers().get(AUTHORIZATION) else {
            return false;
        };

        match self {
            Authorizer::Basic(token) => token == value,
            Authorizer::Bearer(token) => token == value,
        }
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use http_body_util::Full;

    use super::*;

    #[test]
    fn set_and_verify() {
        let auth = Auth::Basic {
            user: "admin".into(),
            password: "password".into(),
        };
        let authorizer = auth.authorizer();

        let mut req = Request::builder()
            .uri("https://example.com")
            .body(Full::<Bytes>::default())
            .unwrap();

        assert!(!authorizer.authorized(&req));

        auth.apply(&mut req);
        assert!(auth.authorizer().authorized(&req));

        let auth = Auth::Bearer {
            token: "token".into(),
        };
        auth.apply(&mut req);
        assert!(!authorizer.authorized(&req));
    }
}
