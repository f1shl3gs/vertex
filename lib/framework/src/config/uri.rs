use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;

use configurable::Configurable;
use configurable::schema::{SchemaGenerator, SchemaObject};
use http::Uri;
use http::uri::{Authority, PathAndQuery, Scheme};
use percent_encoding::percent_decode_str;
use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::http::Auth;

/// A wrapper for `http::Uri` that implements the serde traits.
/// Authorization credentials, if exist, will be removed from the URI and stored in `auth`.
///
/// For example: `http://user:password@example.com`
#[derive(Clone, Default, Debug)]
pub struct UriSerde {
    pub uri: Uri,
    pub auth: Option<Auth>,
}

impl Configurable for UriSerde {
    fn generate_schema(generator: &mut SchemaGenerator) -> SchemaObject {
        let mut schema = String::generate_schema(generator);

        schema.format = Some("uri");
        schema.metadata.examples = vec![serde_json::Value::String(
            "http://username:password@example.com/some/resource".to_string(),
        )];

        schema
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
        let path = format!("{self_path}/{other_path}");
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
                let authority = format!("{user}:{password}@{authority}");
                let authority =
                    Authority::from_maybe_shared(authority).map_err(|_| std::fmt::Error)?;
                let mut parts = self.uri.clone().into_parts();
                parts.authority = Some(authority);
                Display::fmt(&Uri::from_parts(parts).unwrap(), f)
            }

            _ => Display::fmt(&self.uri, f),
        }
    }
}

struct UriVisitor;

impl Visitor<'_> for UriVisitor {
    type Value = UriSerde;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        write!(formatter, "a string containing a valid HTTP Uri")
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
    match authority.as_str().split_once('@') {
        // http://example.com:123/path
        None => (authority.clone(), None),
        Some((s, _)) => {
            match s.split_once(':') {
                // http://username@example.com:123/path
                None => {
                    let authority = Authority::from_str(authority.host()).unwrap();

                    (
                        authority,
                        Some(Auth::Basic {
                            user: s.to_string(),
                            password: "".into(),
                        }),
                    )
                }
                // http://username:password@example.com:123/path
                Some((username, password)) => {
                    let authority = Authority::from_str(authority.host()).unwrap();

                    let password = percent_decode_str(password)
                        .decode_utf8_lossy()
                        .to_string()
                        .into();

                    (
                        authority,
                        Some(Auth::Basic {
                            user: username.to_string(),
                            password,
                        }),
                    )
                }
            }
        }
    }
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
                        password: password.to_owned().into(),
                    }
                }),
                "input: {input}"
            );
        }
    }
}
