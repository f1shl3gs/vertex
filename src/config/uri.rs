use std::str::FromStr;
use http::Uri;
use http::uri::{Authority, PathAndQuery, Scheme};
use percent_encoding::percent_decode_str;
use crate::http::Auth;

/// A wrapper for `http::Uri` that implements the serde traits.
/// Authorization credentials, if exist, will be removed from the URI and stored in `auth`.
/// For example: `http:?/user:password@example.com`
#[derive(Clone, Default, Debug)]
pub struct UriWithAuth {
    pub uri: Uri,
    pub auth: Option<Auth>,
}

impl UriWithAuth {
    /// `Uri` supports incomplete URIs such as "/test", "example.com", etc.
    /// This function fills in empty scheme with HTTP, and empty authority
    /// with `127.0.0.1`.
    pub fn with_default_parts(&self) -> Self {
        let mut parts = self.uri.clone()
            .into_parts();

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

        let uri = Uri::from_parts(parts)
            .expect("invalid parts");

        Self {
            uri,
            auth: self.auth.clone(),
        }
    }
}

impl FromStr for UriWithAuth {
    type Err = <Uri as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<Uri>()
            .map(Into::into)
    }
}

impl From<Uri> for UriWithAuth {
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
    let mut url = url::Url::parse(&format!("http://{}", authority))
        .expect("invalid authority");

    let user = url.username();
    if !user.is_empty() {
        let user = percent_decode_str(user)
            .decode_utf8_lossy()
            .into_owned();

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