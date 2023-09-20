use configurable::Configurable;
use hyper::http::uri::InvalidUri;
use hyper_proxy::Custom;
use hyper_proxy::{Intercept, Proxy, ProxyConnector};
use no_proxy::NoProxy;
use serde::{Deserialize, Serialize};

use super::{default_true, skip_serializing_if_default};

fn from_env(key: &str) -> Option<String> {
    // use lowercase first and the uppercase
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
                    self.0.matches(host)
                        || port.map_or(false, |port| {
                            let url = format!("{}:{}", host, port);
                            self.0.matches(&url)
                        })
                });

                // only intercept those that don't match
                !matches
            },
        ))
    }
}

/// Proxy configuration.
///
/// Configure to proxy traffic through an HTTP(S) proxy when making external requests.
///
/// Similar to common proxy configuration convention, users can set different proxies
/// to use based on the type of traffic being proxied, as well as set specific hosts that
/// should not be proxied.
#[derive(Configurable, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProxyConfig {
    #[serde(
        default = "default_true",
        skip_serializing_if = "skip_serializing_if_default"
    )]
    pub enabled: bool,

    /// The URL to proxy HTTP requests through.
    #[serde(default)]
    pub http: Option<String>,

    /// The URL to proxy HTTPS requests through.
    #[serde(default)]
    pub https: Option<String>,

    /// A list of hosts to avoid proxying. Allowed patterns here include:
    ///
    /// Domain names:     "example.com" matches requests to example.com
    /// Wildcard domains: ".example.com" matches requests to example.com and its
    ///                    subdomains
    /// IP address:        "127.0.0.1" matches requests to 127.0.0.1
    /// CIDR blocks:       "192.168.0.0./16" matches requests to any IP addresses in this range.
    /// Splat:             "*" matches all hosts
    #[serde(default, skip_serializing_if = "skip_serializing_if_default")]
    pub no_proxy: NoProxy,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            http: None,
            https: None,
            no_proxy: NoProxy::default(),
        }
    }
}

impl ProxyConfig {
    pub fn from_env() -> Self {
        Self {
            enabled: true,
            http: from_env("HTTP_PROXY"),
            https: from_env("HTTPS_PROXY"),
            no_proxy: from_env("NO_PROXY").map(NoProxy::from).unwrap_or_default(),
        }
    }

    pub fn merge_with_env(global: &Self, component: &Self) -> Self {
        Self::from_env().merge(&global.merge(component))
    }

    fn interceptor(&self) -> NoProxyInterceptor {
        NoProxyInterceptor(self.no_proxy.clone())
    }

    // overrides current proxy configuration with other configuration
    // if `self` is the global config and `other` is the component config,
    // if both have the `http` proxy set, the one from `other` should be kept
    pub fn merge(&self, other: &Self) -> Self {
        let no_proxy = if other.no_proxy.is_empty() {
            self.no_proxy.clone()
        } else {
            other.no_proxy.clone()
        };

        Self {
            enabled: self.enabled && other.enabled,
            http: other.http.clone().or_else(|| self.http.clone()),
            https: other.https.clone().or_else(|| self.https.clone()),
            no_proxy,
        }
    }

    fn http_intercept(&self) -> Intercept {
        self.interceptor().intercept("http")
    }

    fn http_proxy(&self) -> Result<Option<Proxy>, InvalidUri> {
        self.http
            .as_ref()
            .map(|url| {
                url.parse()
                    .map(|parsed| Proxy::new(self.http_intercept(), parsed))
            })
            .transpose()
    }

    fn https_intercept(&self) -> Intercept {
        self.interceptor().intercept("https")
    }

    fn https_proxy(&self) -> Result<Option<Proxy>, InvalidUri> {
        self.https
            .as_ref()
            .map(|url| {
                url.parse()
                    .map(|parsed| Proxy::new(self.https_intercept(), parsed))
            })
            .transpose()
    }

    /// Install the [`ProxyConnector<C>`] for this `ProxyConfig`
    ///
    /// # Error
    ///
    /// Function will error if passed `ProxyConnector` has a faulty URI
    pub fn configure<C>(&self, connector: &mut ProxyConnector<C>) -> Result<(), InvalidUri> {
        if self.enabled {
            if let Some(proxy) = self.http_proxy()? {
                connector.add_proxy(proxy);
            }

            if let Some(proxy) = self.https_proxy()? {
                connector.add_proxy(proxy);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_simple() {
        let first = ProxyConfig::default();
        let second = ProxyConfig {
            https: Some("https://1.2.3.4:5678".into()),
            ..Default::default()
        };

        let result = first.merge(&second);
        assert_eq!(result.http, None);
        assert_eq!(result.https, Some("https://1.2.3.4:5678".into()))
    }

    #[test]
    fn merge_fill() {
        // coming from env
        let first = ProxyConfig {
            http: Some("http://1.2.3.4:5678".into()),
            ..Default::default()
        };
        // global config
        let second = ProxyConfig {
            https: Some("https://2.3.4.5:6789".into()),
            ..Default::default()
        };
        // component config
        let third = ProxyConfig {
            no_proxy: NoProxy::from("localhost"),
            ..Default::default()
        };

        let result = first.merge(&second).merge(&third);
        assert_eq!(result.http, Some("http://1.2.3.4:5678".into()));
        assert_eq!(result.https, Some("https://2.3.4.5:6789".into()));
        assert!(result.no_proxy.matches("localhost"));
    }

    #[test]
    fn merge_override() {
        let first = ProxyConfig {
            http: Some("http://1.2.3.4:5678".into()),
            no_proxy: NoProxy::from("127.0.0.1,google.com"),
            ..Default::default()
        };
        let second = ProxyConfig {
            http: Some("http://1.2.3.4:5678".into()),
            https: Some("https://2.3.4.5:6789".into()),
            no_proxy: NoProxy::from("localhost"),
            ..Default::default()
        };
        let result = first.merge(&second);
        assert_eq!(result.http, Some("http://1.2.3.4:5678".into()));
        assert_eq!(result.https, Some("https://2.3.4.5:6789".into()));
        assert!(!result.no_proxy.matches("127.0.0.1"));
        assert!(result.no_proxy.matches("localhost"));
    }

    #[test]
    fn with_environment_variables() {
        let global_proxy = ProxyConfig {
            http: Some("http://1.2.3.4:5678".into()),
            ..Default::default()
        };
        let component_proxy = ProxyConfig {
            https: Some("https://2.3.4.5:6789".into()),
            ..Default::default()
        };
        std::env::set_var("HTTP_PROXY", "http://remote.proxy");
        std::env::remove_var("HTTPS_PROXY");

        let result = ProxyConfig::merge_with_env(&global_proxy, &component_proxy);

        assert_eq!(result.http, Some("http://1.2.3.4:5678".into()));
        assert_eq!(result.https, Some("https://2.3.4.5:6789".into()));

        // with the component proxy disabled
        let global_proxy = ProxyConfig {
            https: Some("https://2.3.4.5:6789".into()),
            ..Default::default()
        };
        let component_proxy = ProxyConfig {
            enabled: false,
            ..Default::default()
        };
        let result = ProxyConfig::merge_with_env(&global_proxy, &component_proxy);
        assert!(!result.enabled);
        assert_eq!(result.http, Some("http://remote.proxy".into()));
        assert_eq!(result.https, Some("https://2.3.4.5:6789".into()));
    }
}
