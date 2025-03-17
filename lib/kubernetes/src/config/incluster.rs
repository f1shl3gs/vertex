use std::env;
use std::fmt::Debug;
use std::path::PathBuf;

use rustls::pki_types::CertificateDer;
use rustls::{ClientConfig, RootCertStore};

use super::{Auth, Config, RefreshableToken};

const SERVICE_HOSTENV: &str = "KUBERNETES_SERVICE_HOST";
const SERVICE_PORTENV: &str = "KUBERNETES_SERVICE_PORT";

// Mounted credential files
const SERVICE_TOKENFILE: &str = "/var/run/secrets/kubernetes.io/serviceaccount.yaml/token";
const SERVICE_CERTFILE: &str = "/var/run/secrets/kubernetes.io/serviceaccount.yaml/ca.crt";
const SERVICE_DEFAULT_NS: &str = "/var/run/secrets/kubernetes.io/serviceaccount.yaml/namespace";

/// Errors from loading in-cluster config
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Failed to read the default namespace for the service account
    #[error("failed to read the default namespace: {0}")]
    ReadDefaultNamespace(#[source] std::io::Error),

    /// Failed to read the in-cluster environment variables
    #[error("failed to read an in-cluster environment variable {0}, {1}")]
    ReadEnvironmentVariable(&'static str, #[source] env::VarError),

    /// Failed to read a certificate
    #[error("failed to read the certificate file {0}")]
    ReadCertificate(#[source] std::io::Error),

    /// Failed to parse cluster port value
    #[error("failed to parse cluster port: {0}")]
    ParseClusterPort(#[source] std::num::ParseIntError),

    /// Failed to parse cluster url
    #[error("failed to parse cluster uri: {0}")]
    ParseClusterUri(#[source] http::uri::InvalidUri),

    /// Failed to parse PEM-encoded certificates
    #[error("failed to parse PEM-encoded certificates: {0}")]
    ParseCertificates(#[source] pem::PemError),

    /// Failed to read token file
    #[error("failed to read token file: '{1:?}': {0}")]
    ReadTokenFile(#[source] std::io::Error, PathBuf),

    #[error("failed to build a RootCertStore: {0}")]
    BuildRootCertStore(#[source] rustls::Error),
}

pub fn incluster_env() -> Result<Config, Error> {
    let cluster_url = try_uri()?;
    let default_namespace = load_default_namespace()?;
    let tls = load_tls()?;
    let refreshable_token = RefreshableToken::new(PathBuf::from(SERVICE_TOKENFILE))
        .map_err(|err| Error::ReadTokenFile(err, SERVICE_TOKENFILE.into()))?;

    Ok(Config {
        cluster_url,
        default_namespace,
        auth: Auth::RefreshableToken(refreshable_token),
        proxy_url: None,
        tls,
    })
}

/// Returns the URI of the Kubernetes API server by reading the
/// `KUBERNETES_SERVICE_HOST` and `KUBERNETES_SERVICE_PORT` environment
/// variables.
fn try_uri() -> Result<http::Uri, Error> {
    let host = env::var(SERVICE_HOSTENV)
        .map_err(|err| Error::ReadEnvironmentVariable(SERVICE_HOSTENV, err))?;
    let port = env::var(SERVICE_PORTENV)
        .map_err(|err| Error::ReadEnvironmentVariable(SERVICE_PORTENV, err))?
        .parse::<u16>()
        .map_err(Error::ParseClusterPort)?;

    // Format a host and, if not using 443, a port.
    //
    // Ensure that IPv6 addresses are properly bracketed.
    const HTTPS: &str = "https";

    let uri = match host.parse::<std::net::IpAddr>() {
        Ok(ip) => {
            if port == 443 {
                if ip.is_ipv6() {
                    format!("{HTTPS}://[{ip}]")
                } else {
                    format!("{HTTPS}://{ip}")
                }
            } else {
                format!("{HTTPS}://{ip}:{port}")
            }
        }
        Err(_err) => {
            if port == 443 {
                format!("{HTTPS}://{host}")
            } else {
                format!("{HTTPS}://{host}:{port}")
            }
        }
    };

    uri.parse().map_err(Error::ParseClusterUri)
}

/// Returns the default namespace from specified path in cluster.
fn load_default_namespace() -> Result<String, Error> {
    std::fs::read_to_string(SERVICE_DEFAULT_NS).map_err(Error::ReadDefaultNamespace)
}

/// Returns certification from specified path in cluster
fn load_tls() -> Result<ClientConfig, Error> {
    let data = std::fs::read(SERVICE_CERTFILE).map_err(Error::ReadCertificate)?;
    let certs = pem::parse_many(data)
        .map_err(Error::ParseCertificates)?
        .into_iter()
        .filter_map(|p| {
            if p.tag() == "CERTIFICATE" {
                Some(p.into_contents())
            } else {
                None
            }
        });

    let mut root_store = RootCertStore::empty();
    for cert in certs {
        root_store
            .add(CertificateDer::from(cert))
            .map_err(Error::BuildRootCertStore)?;
    }

    let config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    Ok(config)
}
