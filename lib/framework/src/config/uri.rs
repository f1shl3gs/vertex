use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;

use configurable::schema::{generate_string_schema, SchemaGenerator, SchemaObject};
use configurable::{Configurable, GenerateError};
use http::uri::{Authority, PathAndQuery, Scheme};
use http::Uri;
use percent_encoding::percent_decode_str;
use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::http::Auth;

/// A wrapper for `http::Uri` that implements the serde traits.
/// Authorization credentials, if exist, will be removed from the URI and stored in `auth`.
/// For example: `http:?/user:password@example.com`
#[derive(Clone, Default, Debug)]
pub struct UriSerde {
    pub uri: Uri,
    pub auth: Option<Auth>,
}

impl Configurable for UriSerde {
    fn generate_schema(_gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        let mut schema = generate_string_schema();
        schema.format = Some("uri".to_string());

        let metadata = schema.metadata();
        metadata.examples = vec![serde_json::Value::String(
            "http://username:password@example.com/some/resource".to_string(),
        )];

        Ok(schema)
    }
}

impl UriSerde {
    /// `Uri` supports incomplete URIs such as "/test", "example.com", etc.
    /// This function fills in empty scheme with HTTP, and empty authority
    /// with `127.0.0.1`.
    pub fn with_default_parts(&self) -> Self {
        let mut parts = self.uri.clone().into_parts();

        if parts.scheme.is_none() {
            parts.scheme = Some(Scheme::HTTP);
        }

        if parts.authority.is_none() {
            parts.authority = Some(Authority::from_static("127.0.0.1"));
        }

        if parts.path_and_query.is_none() {
            // just an empty `path_and_query`, but `from_parts` will fail without this.
            parts.path_and_query = Some(PathAndQuery::from_static(""));
        }

        let uri = Uri::from_parts(parts).expect("invalid parts");

        Self {
            uri,
            auth: self.auth.clone(),
        }
    }

    /// Creates a new instance of `UriWithAuth` by appending a path to the existing one.
    pub fn append_path(&self, path: &str) -> crate::Result<Self> {
        let uri = self.uri.to_string();
        let self_path = uri.trim_end_matches('/');
        let other_path = path.trim_start_matches('/');
        let path = format!("{}/{}", self_path, other_path);
        let uri = path.parse::<Uri>()?;
        Ok(Self {
            uri,
            auth: self.auth.clone(),
        })
    }

    pub fn with_auth(mut self, auth: Option<Auth>) -> Self {
        self.auth = auth;
        self
    }
}

impl Serialize for UriSerde {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for UriSerde {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(UriVisitor)
    }
}

impl Display for UriSerde {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match (self.uri.authority(), &self.auth) {
            (Some(authority), Some(Auth::Basic { user, password })) => {
                let authority = format!("{}:{}@{}", user, password, authority);
                let authority =
                    Authority::from_maybe_shared(authority).map_err(|_| std::fmt::Error)?;
                let mut parts = self.uri.clone().into_parts();
                parts.authority = Some(authority);
                std::fmt::Display::fmt(&Uri::from_parts(parts).unwrap(), f)
            }

            _ => std::fmt::Display::fmt(&self.uri, f),
        }
    }
}

struct UriVisitor;

impl<'a> Visitor<'a> for UriVisitor {
    type Value = UriSerde;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        write!(formatter, "a string containning a valid HTTP Uri")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        v.parse().map_err(serde::de::Error::custom)
    }
}

impl FromStr for UriSerde {
    type Err = <Uri as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<Uri>().map(Into::into)
    }
}

impl From<Uri> for UriSerde {
    fn from(uri: Uri) -> Self {
        match uri.authority() {
            None => Self { uri, auth: None },
            Some(authority) => {
                let (authority, auth) = get_basic_auth(authority);

                let mut parts = uri.into_parts();
                parts.authority = Some(authority);
                let uri = Uri::from_parts(parts).unwrap();

                Self { auth, uri }
            }
        }
    }
}

fn get_basic_auth(authority: &Authority) -> (Authority, Option<Auth>) {
    // We get a valid `Authority` as input, therefore cannot fail here.
    let mut url = url::Url::parse(&format!("http://{}", authority)).expect("invalid authority");

    let user = url.username();
    if !user.is_empty() {
        let user = percent_decode_str(user).decode_utf8_lossy().into_owned();

        let password = url.password().unwrap_or("");
        let password = percent_decode_str(password)
            .decode_utf8_lossy()
            .into_owned();

        // These methods have the same failure condition as `username`,
        // because we have a non-empty username, they cannot fail here.
        url.set_username("").expect("unexpected empty authority");
        url.set_password(None).expect("unexpected authority");

        // We get a valid `Authority` as input, therefore cannot fail here.
        let authority = Uri::from_maybe_shared(String::from(url))
            .expect("invalid url")
            .authority()
            .expect("unexpected empty authority")
            .clone();

        (authority, Some(Auth::Basic { user, password }))
    } else {
        (authority.clone(), None)
    }
}

pub fn protocol_endpoint(uri: Uri) -> (String, String) {
    let mut parts = uri.into_parts();

    // Drop any username and password
    parts.authority = parts.authority.map(|auth| {
        let host = auth.host();
        match auth.port() {
            None => host.to_string(),
            Some(port) => format!("{}:{}", host, port),
        }
        .parse()
        .unwrap_or_else(|_| unreachable!())
    });

    // Drop the query and fragment
    parts.path_and_query = parts.path_and_query.map(|pq| {
        pq.path()
            .parse::<PathAndQuery>()
            .unwrap_or_else(|_| unreachable!())
    });

    (
        parts.scheme.clone().unwrap_or(Scheme::HTTP).as_str().into(),
        Uri::from_parts(parts)
            .unwrap_or_else(|_| unreachable!())
            .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_endpoint() {
        let tests = [
            (
                "http://user:pass@example.com/test",
                "http://example.com/test",
                Some(("user", "pass")),
            ),
            ("localhost:8080", "localhost:8080", None),
            ("/api/test", "/api/test", None),
            (
                "http://user:pass;@example.com",
                "http://example.com",
                Some(("user", "pass;")),
            ),
            (
                "user:pass@example.com",
                "example.com",
                Some(("user", "pass")),
            ),
            ("user@example.com", "example.com", Some(("user", ""))),
        ];

        for (input, want_uri, want_auth) in tests {
            let UriSerde { uri, auth } = input.parse().unwrap();
            assert_eq!(uri, Uri::from_maybe_shared(want_uri).unwrap());
            assert_eq!(
                auth,
                want_auth.map(|(user, password)| {
                    Auth::Basic {
                        user: user.to_owned(),
                        password: password.to_owned(),
                    }
                })
            );
        }
    }

    #[test]
    fn protocol_endpoint_parses_urls() {
        let tests = [
            ("http://example.com/", "http", "http://example.com/"),
            (
                "https://user:pass@example.org:123/path?query",
                "https",
                "https://example.org:123/path",
            ),
            (
                "gopher://example.net:123/path?foo=bar#frag,emt",
                "gopher",
                "gopher://example.net:123/path",
            ),
        ];

        for (input, protocol, endpoint) in tests {
            assert_eq!(
                protocol_endpoint(input.parse().unwrap()),
                (protocol.into(), endpoint.into())
            );
        }
    }
}
