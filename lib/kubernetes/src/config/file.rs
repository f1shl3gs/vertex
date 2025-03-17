use std::path::{Path, PathBuf};
use std::sync::Arc;

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, DigitallySignedStruct, RootCertStore, SignatureScheme};
use rustls_native_certs::CertificateResult;
use serde::Deserialize;
use tracing::debug;

use super::tls::client_auth;
use super::{Auth, Config, LoadDataError, RefreshableToken};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Failed to read kube config
    #[error("failed to read '{1:?}': {0}")]
    ReadFile(#[source] std::io::Error, PathBuf),
    /// Failed to parse kube config YAML
    #[error("failed to parse kube config YAML: {0}")]
    Parse(#[source] serde_yaml::Error),
    /// Failed to determine current context
    #[error("failed to determine current context")]
    CurrentContextNotSet,
    /// Failed to load current context
    #[error("failed to load current context: {0}")]
    LoadContext(String),
    /// Failed to load the cluster of context
    #[error("failed to load the cluster of context: {0}")]
    LoadClusterOfContext(String),
    /// Failed to find named user
    #[error("failed to find named user: {0}")]
    FindUser(String),
    /// Cluster url is missing on selected cluster
    #[error("cluster url is missing on selected cluster")]
    MissingClusterUrl,
    /// Failed to parse cluster uri
    #[error("failed to parse cluster url: {0}")]
    ParseClusterUri(#[source] http::uri::InvalidUri),
    #[error("build tls config failed, {0}")]
    Tls(#[from] super::tls::Error),
    /// Failed to load client certificate
    #[error("failed to load client certificate")]
    LoadClientCertificate(#[source] LoadDataError),
    /// Failed to load client key
    #[error("failed to load client key")]
    LoadClientKey(#[source] LoadDataError),
    /// Failed to load certificate authority
    #[error("failed to load certificate authority")]
    LoadCertificateAuthority(#[source] LoadDataError),
    /// Failed to parse PEM-encoded certificates
    #[error("failed to parse PEM-encoded certificates: {0}")]
    ParseCertificates(#[source] pem::PemError),
    /// Load native certificates failed
    #[error("load native certificates: {0:?}")]
    LoadNativeCertificates(Vec<rustls_native_certs::Error>),
    /// Invalid private key
    #[error("invalid private key: {0}")]
    InvalidPrivateKey(#[source] rustls::Error),
    /// Failed to add a root certificate
    #[error("failed to add a root certificate: {0}")]
    AddRootCertificate(#[source] rustls::Error),
}

#[derive(Clone, Debug, Default, Deserialize)]
struct AuthInfo {
    /// The username for basic authentication to the kubernetes cluster.
    pub username: Option<String>,
    /// the password for basic authentication to the kubernetes cluster.
    pub password: Option<String>,

    /// The bearer token for authentication to the kubernetes cluster.
    pub token: Option<String>,
    /// Pointer to a file that contains a bearer token (as described above).
    pub token_file: Option<PathBuf>,

    /// Path to a client cert file for TLS.
    #[serde(rename = "client-certificate")]
    pub client_certificate: Option<PathBuf>,
    /// PEM-encoded data from a client cert file for TLS. Overrides `client_certificate`
    #[serde(rename = "client-certificate-data")]
    pub client_certificate_data: Option<String>,

    /// Path to a client key file for TLS
    #[serde(rename = "client-key")]
    pub client_key: Option<PathBuf>,
    /// PEM-encoded data from a client key file for TLS. Overrides `client_key`
    #[serde(rename = "client-key-data")]
    pub client_key_data: Option<String>,
}

/// NamedAuthInfo associates name with authentication.
#[derive(Deserialize)]
struct NamedAuthInfo {
    /// Name of the user
    name: String,

    /// Information that describes identity of the user
    #[serde(rename = "user")]
    auth_info: Option<AuthInfo>,
}

/// Cluster stores information to connect Kubernetes cluster.
#[derive(Clone, Deserialize)]
struct Cluster {
    /// The address of the kubernetes cluster (https://hostname:port)
    server: Option<String>,

    /// Skips the validity check for the server's certificate. This will make your HTTPS
    /// connections insecure.
    #[serde(rename = "insecure-skip-tls-verify", default)]
    insecure_skip_tls_verify: bool,

    /// The path to a cert file for the certificate authority.
    #[serde(rename = "certificate-authority")]
    certificate_authority: Option<PathBuf>,

    /// PEM-encoded certificate authority certificates. Overrides `certificate_authority`
    #[serde(rename = "certificate-authority-data")]
    certificate_authority_data: Option<String>,
    // /// URL to the proxy to be used for all requests.
    // #[serde(rename = "proxy-url")]
    // proxy_url: Option<String>,
}

/// NamedCluster associates name with cluster.
#[derive(Deserialize)]
struct NamedCluster {
    /// Name of cluster
    name: String,

    /// Information about how to communicate with  a kubernetes cluster.
    cluster: Option<Cluster>,
}

/// Context stores tuple of cluster and user information.
#[derive(Clone, Deserialize)]
struct Context {
    /// Name of the cluster for this context.
    cluster: String,

    /// Name of the `AuthInfo` for this context.
    user: String,

    /// The default namespace to use on unspecified requests
    namespace: Option<String>,
}

/// NamedContext associates name with context.
#[derive(Deserialize)]
struct NamedContext {
    /// Name of the context
    name: String,

    /// Associations for the context
    context: Option<Context>,
}

/// [`KubeConfig`] represents information on how to connect to a remote
/// Kubernetes cluster.
///
/// NOTE: Only necessary fields are present here.
///
/// Stored in `~/.kube/config` by default, but can be distributed across
/// multiple paths in passed through `KUBECONFIG`.
/// An analogue of the [config type from client-go](https://github.com/kubernetes/client-go/blob/7697067af71046b18e03dbda04e01a5bb17f9809/tools/clientcmd/api/types.go).
#[derive(Deserialize)]
struct KubeConfig {
    /// Referencable names to cluster configs
    clusters: Vec<NamedCluster>,

    /// Referencable names to user configs
    #[serde(rename = "users")]
    auth_infos: Vec<NamedAuthInfo>,

    /// Referencable names to context configs
    contexts: Vec<NamedContext>,

    /// The name of the context that you would like to use by default
    #[serde(rename = "current-context")]
    current_context: Option<String>,
}

pub fn from_config(path: impl AsRef<Path>) -> Result<Config, Error> {
    let path = path.as_ref();
    let data = std::fs::read(path).map_err(|err| Error::ReadFile(err, path.into()))?;
    let config = serde_yaml::from_slice::<KubeConfig>(&data).map_err(Error::Parse)?;

    let context_name = config.current_context.ok_or(Error::CurrentContextNotSet)?;
    let context = config
        .contexts
        .iter()
        .find(|ctx| ctx.name == context_name)
        .and_then(|ctx| ctx.context.clone())
        .ok_or_else(|| Error::LoadContext(context_name))?;
    let cluster = config
        .clusters
        .iter()
        .find(|cluster| cluster.name == context.cluster)
        .and_then(|named_cluster| named_cluster.cluster.clone())
        .ok_or_else(|| Error::LoadClusterOfContext(context.cluster.clone()))?;
    let auth_info = config
        .auth_infos
        .iter()
        .find(|named_user| named_user.name == context.user)
        .and_then(|named_user| named_user.auth_info.clone())
        .ok_or_else(|| Error::FindUser(context.user))?;
    let cluster_url = cluster
        .server
        .ok_or(Error::MissingClusterUrl)?
        .parse::<http::Uri>()
        .map_err(Error::ParseClusterUri)?;
    let default_namespace = context.namespace.unwrap_or_else(|| String::from("default"));

    // build tls config
    let client_cert = load_base64_or_file(
        auth_info.client_certificate_data.as_ref(),
        auth_info.client_certificate.as_ref(),
    )
    .map_err(Error::LoadClientCertificate)?;
    let client_key = load_base64_or_file(
        auth_info.client_key_data.as_ref(),
        auth_info.client_key.as_ref(),
    )
    .map_err(Error::LoadClientKey)?;

    let mut identity_pem = client_key;
    identity_pem.extend_from_slice(&client_cert);

    let root_certs = if cluster.certificate_authority.is_none()
        && cluster.certificate_authority_data.is_none()
    {
        None
    } else {
        let data = load_base64_or_file(
            cluster.certificate_authority_data.as_ref(),
            cluster.certificate_authority.as_ref(),
        )
        .map_err(Error::LoadCertificateAuthority)?;
        let certs = pem::parse_many(data)
            .map_err(Error::ParseCertificates)?
            .into_iter()
            .filter_map(|p| {
                if p.tag() == "CERTIFICATE" {
                    Some(p.into_contents())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        Some(certs)
    };

    let root_store = if let Some(certs) = root_certs {
        root_store(certs)?
    } else {
        let CertificateResult { certs, errors, .. } = rustls_native_certs::load_native_certs();
        if !errors.is_empty() {
            return Err(Error::LoadNativeCertificates(errors));
        }

        let mut root_store = RootCertStore::empty();
        for cert in certs {
            if let Err(err) = root_store.add(cert) {
                debug!(
                    message = "certificate parse failed",
                    %err
                );
            }
        }

        if root_store.is_empty() {
            debug!(message = "no valid native root CA certificates found");
        }

        root_store
    };

    let (chain, pkey) = client_auth(&identity_pem)?;
    let mut tls = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_client_auth_cert(chain, pkey)
        .map_err(Error::InvalidPrivateKey)?;
    if cluster.insecure_skip_tls_verify {
        tls.dangerous()
            .set_certificate_verifier(Arc::new(NoCertificateVerification));
    }

    let auth = if let (Some(username), Some(password)) = (auth_info.username, auth_info.password) {
        Auth::Basic { username, password }
    } else if let Some(path) = auth_info.token_file {
        let refreshable_token =
            RefreshableToken::new(path.clone()).map_err(|err| Error::ReadFile(err, path))?;

        Auth::RefreshableToken(refreshable_token)
    } else if let Some(token) = auth_info.token {
        Auth::Bearer { token }
    } else {
        Auth::None
    };

    Ok(Config {
        cluster_url,
        default_namespace,
        auth,
        proxy_url: None,
        tls,
    })
}

fn load_base64_or_file(
    data: Option<&String>,
    file: Option<&PathBuf>,
) -> Result<Vec<u8>, LoadDataError> {
    if let Some(data) = data {
        return decode_base64(data);
    }

    match file {
        Some(path) => {
            let data =
                std::fs::read(path).map_err(|err| LoadDataError::ReadFile(err, path.clone()))?;
            decode_base64(data)
        }
        None => Err(LoadDataError::MissingDataOrFile),
    }
}

#[inline]
fn decode_base64(value: impl AsRef<[u8]>) -> Result<Vec<u8>, LoadDataError> {
    use base64::Engine;

    base64::engine::general_purpose::STANDARD
        .decode(value)
        .map_err(LoadDataError::DecodeBase64)
}

fn root_store(root_certs: Vec<Vec<u8>>) -> Result<RootCertStore, Error> {
    let mut root_store = RootCertStore::empty();
    for cert in root_certs {
        root_store
            .add(CertificateDer::from(cert))
            .map_err(Error::AddRootCertificate)?;
    }

    Ok(root_store)
}

#[derive(Debug)]
pub struct NoCertificateVerification;

impl ServerCertVerifier for NoCertificateVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer,
        _intermediates: &[CertificateDer],
        _server_name: &ServerName,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        tracing::warn!("Server cert bypassed");
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        use rustls::SignatureScheme;

        vec![
            SignatureScheme::RSA_PKCS1_SHA1,
            SignatureScheme::ECDSA_SHA1_Legacy,
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
            SignatureScheme::ECDSA_NISTP521_SHA512,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
            SignatureScheme::ED25519,
            SignatureScheme::ED448,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize() {
        let data = r#"
apiVersion: v1
clusters:
- cluster:
    certificate-authority-data: LS0tLS1CRUdJTiBDRVJ
    server: https://127.0.0.1:34139
  name: kind-kind
contexts:
- context:
    cluster: kind-kind
    user: kind-kind
  name: kind-kind
current-context: kind-kind
kind: Config
preferences: {}
users:
- name: kind-kind
  user:
    client-certificate-data: LS0tLS1CRUdJTiBDRVJUSUZ
    client-key-data: LS0tLS1CRUdJTiBSU0EgUFJJVkFURSB
"#;
        let config = serde_yaml::from_str::<KubeConfig>(data).unwrap();

        assert_eq!(config.clusters.len(), 1);
        assert_eq!(config.clusters[0].name, "kind-kind");
        let cluster = config.clusters.first().unwrap().cluster.as_ref().unwrap();
        assert_eq!(cluster.server.as_ref().unwrap(), "https://127.0.0.1:34139");
        assert_eq!(
            cluster.certificate_authority_data.as_ref().unwrap(),
            "LS0tLS1CRUdJTiBDRVJ"
        );

        assert_eq!(config.auth_infos.len(), 1);
        let auth_info = config.auth_infos.first().unwrap();
        assert_eq!(auth_info.name, "kind-kind");
        let user = auth_info.auth_info.as_ref().unwrap();
        assert_eq!(
            user.client_certificate_data.as_ref().unwrap(),
            "LS0tLS1CRUdJTiBDRVJUSUZ"
        );
        assert_eq!(
            user.client_key_data.as_ref().unwrap(),
            "LS0tLS1CRUdJTiBSU0EgUFJJVkFURSB"
        );
    }
}
