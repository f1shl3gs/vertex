use std::path::PathBuf;
use std::sync::Arc;
use std::{fs, io};

use configurable::Configurable;
use hyper_rustls::ConfigBuilderExt;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName, UnixTime};
use rustls::server::WebPkiClientVerifier;
use rustls::{ClientConfig, DigitallySignedStruct, Error, SignatureScheme};
use rustls::{RootCertStore, ServerConfig};
use serde::{Deserialize, Serialize};

use super::TlsError;
use crate::config::default_true;

/// Configures the TLS options for incoming/outgoing connections.
#[derive(Configurable, Clone, Debug, Deserialize, Serialize)]
pub struct TlsConfig {
    /// Absolute path to an additional CA certificate file, in DER or PEM
    /// format(X.509), or an inline CA certificate in PEM format.
    pub ca: Option<PathBuf>,

    /// Absolute path to a certificate file used to identify this connection,
    /// in DER or PEM format (X.509) or PKCS#12, or an inline certificate in
    /// PEM format. If this is set and is not a PKCS#12 archive, "key_file"
    /// must also be set.
    pub cert: Option<PathBuf>,

    /// Absolute path to a private key file used to identify this connection,
    /// in DER or PEM format (PKCS#8), or an inline private key in PEM format.
    /// If this is set, "crt_file" must also be set.
    pub key: Option<PathBuf>,

    /// Pass phrase used to unlock the encrypted key file. This has no effect
    /// unless "key" is set.
    pub key_pass: Option<String>,

    /// Enables certificate verification.
    /// If enabled, certificates must not be expired and must be issued by a trusted issuer.
    /// This verification operates in a hierarchical manner, checking that the leaf certificate
    /// (the certificate presented by the client/server) is not only valid, but that the issuer
    /// of that certificate is also valid, and so on until the verification process reaches a
    /// root certificate.
    ///
    /// Relevant for both incoming and outgoing connections.
    ///
    /// Do NOT set this to false unless you understand the risks of not verifying the
    /// validity of certificates.
    #[serde(default = "default_true")]
    pub verify_certificate: bool,

    /// Enables hostname verification. If enabled, the hostname used to connect to the remote
    /// host must be present in the TLS certificate presented by the remote host, either as the
    /// Common Name or as an entry in the Subject Alternative Name extension.
    ///
    /// Only relevant for outgoing connections.
    ///
    /// Do NOT set this to false unless you understand the risks of not verifying the remote hostname.
    #[serde(default = "default_true")]
    pub verify_hostname: bool,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            verify_certificate: true,
            verify_hostname: true,
            ca: None,
            cert: None,
            key: None,
            key_pass: None,
        }
    }
}

impl TlsConfig {
    #[cfg(any(test, feature = "test-util"))]
    pub fn test_server_config() -> Self {
        TlsConfig {
            cert: Some("tests/ca/intermediate/certs/localhost.cert.pem".into()),
            key: Some("tests/ca/intermediate/private/localhost.nopass.key.pem".into()),
            ..TlsConfig::default()
        }
    }

    #[cfg(any(test, feature = "test-util"))]
    pub fn test_client_config() -> Self {
        TlsConfig {
            ca: Some("tests/ca/intermediate/certs/ca-chain.cert.pem".into()),
            ..TlsConfig::default()
        }
    }

    pub fn client_config(&self) -> Result<ClientConfig, TlsError> {
        let builder = if let Some(ca_file) = &self.ca {
            let mut root_store = RootCertStore::empty();
            let certs = load_certs(ca_file)?;
            for cert in certs {
                root_store.add(cert).map_err(TlsError::AddCertToStore)?;
            }

            ClientConfig::builder().with_root_certificates(root_store)
        } else {
            ClientConfig::builder()
                .with_native_roots()
                .map_err(TlsError::NativeCerts)?
        };

        let mut config = match (&self.cert, &self.key) {
            (Some(cert_file), Some(key_file)) => {
                let certs = load_certs(cert_file)?;
                let key = load_private_key(key_file, self.key_pass.as_deref())?;

                builder
                    .with_client_auth_cert(certs, key)
                    .map_err(TlsError::TlsBuild)?
            }
            (Some(_), None) => return Err(TlsError::MissingKey),
            (None, Some(_)) => return Err(TlsError::MissingCertificate),
            (None, None) => builder.with_no_client_auth(),
        };

        if !self.verify_certificate {
            config
                .dangerous()
                .set_certificate_verifier(Arc::new(NoServerCertVerifier))
        }

        Ok(config)
    }

    pub fn server_config(&self) -> Result<ServerConfig, TlsError> {
        let builder = if let Some(ca_file) = &self.ca {
            let certs = load_certs(ca_file)?;
            let mut store = RootCertStore::empty();
            for cert in certs {
                store.add(cert).map_err(TlsError::AddCertToStore)?;
            }

            let client_auth = WebPkiClientVerifier::builder(Arc::new(store))
                .build()
                .map_err(TlsError::VerifierBuild)?;
            ServerConfig::builder().with_client_cert_verifier(client_auth)
        } else {
            ServerConfig::builder().with_no_client_auth()
        };

        let config = match (&self.cert, &self.key) {
            (Some(cert_file), Some(key_file)) => {
                let certs = load_certs(cert_file)?;
                let key = load_private_key(key_file, self.key_pass.as_deref())?;

                builder
                    .with_single_cert(certs, key)
                    .map_err(TlsError::TlsBuild)?
            }
            (Some(_), None) => return Err(TlsError::MissingKey),
            (None, Some(_)) => return Err(TlsError::MissingCertificate),
            (None, None) => return Err(TlsError::MissingCertAndKey),
        };

        Ok(config)
    }
}

#[derive(Debug)]
struct NoServerCertVerifier;

impl ServerCertVerifier for NoServerCertVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
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

fn load_certs(filename: &PathBuf) -> Result<Vec<CertificateDer<'static>>, TlsError> {
    let content = fs::read(filename).map_err(|err| TlsError::FileReadFailed {
        note: "Read cert file",
        filename: filename.clone(),
        err,
    })?;

    let certs = pem::parse_many(content)
        .map_err(|err| TlsError::CertificateParse {
            filename: filename.clone(),
            err: io::Error::new(io::ErrorKind::InvalidData, err),
        })?
        .into_iter()
        .map(|s| CertificateDer::from(s.into_contents()))
        .collect::<Vec<_>>();

    Ok(certs)
}

fn load_private_key(
    filename: &PathBuf,
    password: Option<&str>,
) -> Result<PrivateKeyDer<'static>, TlsError> {
    use pkcs8::der::Decode;

    let expected_tag = match password {
        Some(_) => "ENCRYPTED PRIVATE KEY",
        None => "PRIVATE KEY",
    };

    let content = fs::read(filename).map_err(|err| TlsError::FileReadFailed {
        note: "Read private key file",
        filename: filename.clone(),
        err,
    })?;

    let mut iter = pem::parse_many(content)
        .map_err(|err| TlsError::PrivateKeyParse {
            filename: filename.clone(),
            err: io::Error::new(io::ErrorKind::InvalidData, err),
        })?
        .into_iter()
        .filter(|x| x.tag() == expected_tag)
        .map(|x| x.into_contents());

    let key = match iter.next() {
        Some(key) => match password {
            Some(password) => {
                let encrypted = pkcs8::EncryptedPrivateKeyInfo::from_der(&key).map_err(|err| {
                    TlsError::PrivateKeyParse {
                        filename: filename.clone(),
                        err: io::Error::new(io::ErrorKind::InvalidData, err),
                    }
                })?;
                let decrypted =
                    encrypted
                        .decrypt(password)
                        .map_err(|err| TlsError::PrivateKeyParse {
                            filename: filename.clone(),
                            err: io::Error::new(io::ErrorKind::InvalidData, err),
                        })?;

                PrivateKeyDer::try_from(decrypted.as_bytes().to_owned()).map_err(|err| {
                    TlsError::PrivateKeyParse {
                        filename: filename.clone(),
                        err: io::Error::new(io::ErrorKind::InvalidData, err),
                    }
                })?
            }
            None => PrivateKeyDer::try_from(key).map_err(|err| TlsError::PrivateKeyParse {
                filename: filename.clone(),
                err: io::Error::new(io::ErrorKind::InvalidData, err),
            })?,
        },
        None => {
            return Err(TlsError::PrivateKeyParse {
                filename: filename.clone(),
                err: io::Error::new(
                    io::ErrorKind::InvalidData,
                    "no private key found in PEM file",
                ),
            })
        }
    };

    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PASSWORD: &str = "nopass";

    #[test]
    fn private_key_with_password() {
        let path = "tests/ca/intermediate/private/localhost.key.pem".into();
        load_private_key(&path, Some(PASSWORD)).unwrap();
        load_private_key(&path, None).unwrap_err();
    }

    #[test]
    fn private_key_without_password() {
        let path = "tests/ca/intermediate/private/localhost.nopass.key.pem".into();
        load_private_key(&path, None).unwrap();
        load_private_key(&path, Some(PASSWORD)).unwrap_err();
    }
}
