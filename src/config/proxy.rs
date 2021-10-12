use hyper_proxy::Intercept;
use hyper_proxy::Custom;
use no_proxy::NoProxy;
use serde::{Deserialize, Serialize};

fn from_env(key: &str) -> Option<String> {
    // use lowercase first and the upercase
    std::env::var(key.to_lowercase())
        .ok()
        .or_else(|| std::env::var(key.to_uppercase()).ok())
}

#[derive(Clone, Default, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct NoProxyInterceptor(NoProxy);

impl NoProxyInterceptor {
    fn intercept(self, expected: &'static str) -> Intercept {
        Intercept::Custom(Custom::from(
            move |scheme: Option<&str>, host: Option<&str>, port: Option<u16>| {
                if scheme.is_some() && scheme != Some(expected) {
                    return false;
                }

                let matches = host.map_or(false, |host| {
                    self.0.matches(host) || port.map_or(false, |port| {
                        let url = format!("{}:{}", host, port);
                        self.0.matches(&url)
                    })
                });

                // only intercept those that don't match
                !matches
            }
        ))
    }
}

/// Answers "Is it possible to skip serializing this value, because it's the
/// default?"
#[inline]
pub fn skip_serializing_if_default<E: Default + PartialEq>(e: &E) -> bool {
    e == &E::default()
}


#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProxyConfig {
    #[serde(default)]
    pub http: Option<String>,
    #[serde(default)]
    pub https: Option<String>,
    #[serde(default, skip_serializing_if = "skip_serializing_if_default")]
    pub no_proxy: NoProxy,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            http: None,
            https: None,
            no_proxy: NoProxy::default(),
        }
    }
}

impl ProxyConfig {
    pub fn from_env() -> Self {
        Self {
            http: from_env("HTTP_PROXY"),
            https: from_env("HTTPS_PROXY"),
            no_proxy: from_env("NO_PROXY")
                .map(NoProxy::from)
                .unwrap_or_default(),
        }
    }

    pub fn merge_with_env(global: &Self, component: &Self) -> Self {
        Self::from_env().merge(&global.merge(component))
    }

    fn interceptor(&self) -> NoProxyInterceptor {
        NoProxyInterceptor(self.no_proxy.clone())
    }

    pub fn merge(&self, other: &Self) -> Self {

    }
}