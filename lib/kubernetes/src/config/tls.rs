use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use thiserror::Error;

/// Errors from Rustls
#[derive(Debug, Error)]
pub enum Error {
    /// Identity PEM is invalid
    #[error("identity PEM is invalid: {0}")]
    InvalidIdentityPem(#[source] rustls::pki_types::pem::Error),

    /// Identity PEM is missing a private key: the key must be PKCS8 or RSA/PKCS1
    #[error("identity PEM is missing a private key: the key must be PKCS8 or RSA/PKCS1")]
    MissingPrivateKey,

    /// Identity PEM is missing certificate
    #[error("identity PEM is missing certificate")]
    MissingCertificate,

    /// Invalid private key
    #[error("invalid private key: {0}")]
    InvalidPrivateKey(#[source] rustls::Error),

    /// Unknown private key format
    #[error("unknown private key format")]
    UnknownPrivateKeyFormat,

    /// Failed to add a root certificate
    #[error("failed to add a root certificate: {0}")]
    AddRootCertificate(#[source] rustls::Error),

    /// No valid native root CA certificates found
    #[error("No valid native root CA certificates found")]
    NoValidNativeRootCA(#[source] std::io::Error),
}

pub fn client_auth(
    data: &[u8],
) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>), Error> {
    use rustls::pki_types::pem::{self, SectionKind};

    let mut cert_chain = Vec::new();
    let mut pkcs8_key = None;
    let mut pkcs1_key = None;
    let mut sec1_key = None;
    let mut reader = std::io::Cursor::new(data);
    while let Some((kind, der)) = pem::from_buf(&mut reader).map_err(Error::InvalidIdentityPem)? {
        match kind {
            SectionKind::Certificate => cert_chain.push(der.into()),
            SectionKind::PrivateKey => pkcs8_key = Some(PrivateKeyDer::Pkcs8(der.into())),
            SectionKind::RsaPrivateKey => pkcs1_key = Some(PrivateKeyDer::Pkcs1(der.into())),
            SectionKind::EcPrivateKey => sec1_key = Some(PrivateKeyDer::Sec1(der.into())),
            _ => return Err(Error::UnknownPrivateKeyFormat),
        }
    }

    let private_key = pkcs8_key
        .or(pkcs1_key)
        .or(sec1_key)
        .ok_or(Error::MissingCertificate)?;

    if cert_chain.is_empty() {
        return Err(Error::MissingCertificate);
    }

    Ok((cert_chain, private_key))
}
