use std::{
    fmt,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use configurable::Configurable;
use openssl::{
    pkcs12::{ParsedPkcs12, Pkcs12},
    pkey::{PKey, Private},
    ssl::{ConnectConfiguration, SslContextBuilder, SslVerifyMode},
    stack::Stack,
    x509::{store::X509StoreBuilder, X509},
};
use serde::{Deserialize, Serialize};

use super::{MaybeTls, Result, TlsError};

const PEM_START_MARKER: &str = "-----BEGIN ";

#[cfg(any(test, feature = "test-util"))]
pub const TEST_PEM_CA_PATH: &str = "tests/fixtures/tls/Vertex_CA.crt";
#[cfg(any(test, feature = "test-util"))]
pub const TEST_PEM_CRT_PATH: &str = "tests/fixtures/tls/localhost.crt";
#[cfg(any(test, feature = "test-util"))]
pub const TEST_PEM_KEY_PATH: &str = "tests/fixtures/tls/localhost.key";

impl TlsConfig {
    #[cfg(any(test, feature = "test-util"))]
    pub fn test_config() -> Self {
        TlsConfig::test_options()
    }
}

/// Configures the TLS options for incoming/outgoing connections.
#[derive(Configurable, Clone, Debug, Default, Deserialize, Serialize)]
pub struct TlsConfig {
    pub verify_certificate: Option<bool>,
    /// If "true", Vertex will validate the configured remote host name against
    /// the remote host's TLS certificate. Do NOT set this to false unless you
    /// understand the risks of not verifying the remote hostname.
    pub verify_hostname: Option<bool>,
    /// Absolute path to an additional CA certificate file, in DER or PEM
    /// format(X.509), or an inline CA certificate in PEM format.
    pub ca_file: Option<PathBuf>,
    /// Absolute path to a certificate file used to identify this connection,
    /// in DER or PEM format (X.509) or PKCS#12, or an inline certificate in
    /// PEM format. If this is set and is not a PKCS#12 archive, "key_file"
    /// must also be set.
    pub crt_file: Option<PathBuf>,
    /// Absolute path to a private key file used to identify this connection,
    /// in DER or PEM format (PKCS#8), or an inline private key in PEM format.
    /// If this is set, "crt_file" must also be set.
    pub key_file: Option<PathBuf>,
    /// Pass phrase used to unlock the encrypted key file. This has no effect
    /// unless "key_file" is set.
    pub key_pass: Option<String>,
}

impl TlsConfig {
    #[cfg(any(test, feature = "test-util"))]
    pub fn test_options() -> Self {
        Self {
            ca_file: Some(TEST_PEM_CA_PATH.into()),
            crt_file: Some(TEST_PEM_CRT_PATH.into()),
            key_file: Some(TEST_PEM_KEY_PATH.into()),
            ..Self::default()
        }
    }
}

/// Directly usable settings for TLS connectors
#[derive(Clone, Default)]
pub struct TlsSettings {
    verify_certificate: bool,
    pub(super) verify_hostname: bool,
    authorities: Vec<X509>,
    pub(super) identity: Option<IdentityStore>, // openssl::pkcs12::ParsedPkcs12 doesn't impl Clone yet
}

#[derive(Clone)]
pub struct IdentityStore(Vec<u8>, String);

impl TlsSettings {
    /// Generate a filled out settings struct from the given optional
    /// option set, interpreted as client options. If `options` is
    /// `None`, the result is set to defaults (ie empty).
    pub fn from_options(options: &Option<TlsConfig>) -> Result<Self> {
        Self::from_options_base(options, false)
    }

    pub(super) fn from_options_base(options: &Option<TlsConfig>, for_server: bool) -> Result<Self> {
        let default = TlsConfig::default();
        let options = options.as_ref().unwrap_or(&default);

        if !for_server {
            if options.verify_certificate == Some(false) {
                warn!(
                    "The `verify_certificate` option is DISABLED, this may lead to security vulnerabilities."
                );
            }
            if options.verify_hostname == Some(false) {
                warn!("The `verify_hostname` option is DISABLED, this may lead to security vulnerabilities.");
            }
        }

        Ok(Self {
            verify_certificate: options.verify_certificate.unwrap_or(!for_server),
            verify_hostname: options.verify_hostname.unwrap_or(!for_server),
            authorities: options.load_authorities()?,
            identity: options.load_identity()?,
        })
    }

    fn identity(&self) -> Option<ParsedPkcs12> {
        // This data was test-built previously, so we can just use it
        // here and expect the results will not fail. This can all be
        // reworked when `openssl::pkcs12::ParsedPkcs12` gains the Clone
        // impl.
        self.identity.as_ref().map(|identity| {
            Pkcs12::from_der(&identity.0)
                .expect("Could not build PKCS#12 archive from parsed data")
                .parse(&identity.1)
                .expect("Could not parse stored PKCS#12 archive")
        })
    }

    pub(super) fn apply_context(&self, context: &mut SslContextBuilder) -> Result<()> {
        context.set_verify(if self.verify_certificate {
            SslVerifyMode::PEER | SslVerifyMode::FAIL_IF_NO_PEER_CERT
        } else {
            SslVerifyMode::NONE
        });
        if let Some(identity) = self.identity() {
            context
                .set_certificate(&identity.cert)
                .map_err(TlsError::SetCertificate)?;
            context
                .set_private_key(&identity.pkey)
                .map_err(TlsError::SetPrivateKey)?;
            if let Some(chain) = identity.chain {
                for cert in chain {
                    context
                        .add_extra_chain_cert(cert)
                        .map_err(TlsError::AddExtraChainCert)?;
                }
            }
        }
        if !self.authorities.is_empty() {
            let mut store = X509StoreBuilder::new().map_err(TlsError::NewStoreBuilder)?;
            for authority in &self.authorities {
                store
                    .add_cert(authority.clone())
                    .map_err(TlsError::AddCertToStore)?;
            }
            context
                .set_verify_cert_store(store.build())
                .map_err(TlsError::SetVerifyCert)?;
        } else {
            debug!("Fetching system root certs.");

            #[cfg(windows)]
            load_windows_certs(context).unwrap();

            #[cfg(target_os = "macos")]
            load_mac_certs(context).unwrap();
        }

        Ok(())
    }

    pub fn apply_connect_configuration(&self, connection: &mut ConnectConfiguration) {
        connection.set_verify_hostname(self.verify_hostname);
    }
}

impl TlsConfig {
    fn load_authorities(&self) -> Result<Vec<X509>> {
        match &self.ca_file {
            None => Ok(vec![]),
            Some(filename) => {
                let (data, filename) = open_read(filename, "certificate")?;
                der_or_pem(
                    data,
                    |der| X509::from_der(&der).map(|x509| vec![x509]),
                    |pem| {
                        pem.match_indices(PEM_START_MARKER)
                            .map(|(start, _)| X509::from_pem(pem[start..].as_bytes()))
                            .collect()
                    },
                )
                .map_err(|err| TlsError::X509Parse { filename, err })
            }
        }
    }

    fn load_identity(&self) -> Result<Option<IdentityStore>> {
        match (&self.crt_file, &self.key_file) {
            (None, Some(_)) => Err(TlsError::MissingCrtKeyFile),
            (None, None) => Ok(None),
            (Some(filename), _) => {
                let (data, filename) = open_read(filename, "certificate")?;
                der_or_pem(
                    data,
                    |der| self.parse_pkcs12_identity(der),
                    |pem| self.parse_pem_identity(pem, &filename),
                )
            }
        }
    }

    /// Parse identity from a PEM encoded certificate + key pair of files
    fn parse_pem_identity(&self, pem: String, crt_file: &Path) -> Result<Option<IdentityStore>> {
        match &self.key_file {
            None => Err(TlsError::MissingKey),
            Some(key_file) => {
                let name = crt_file.to_string_lossy().to_string();
                let mut crt_stack = X509::stack_from_pem(pem.as_bytes())
                    .map_err(|err| TlsError::X509Parse {
                        filename: crt_file.into(),
                        err,
                    })?
                    .into_iter();

                let crt = crt_stack.next().ok_or(TlsError::MissingCertificate)?;
                let key = load_key(key_file, &self.key_pass)?;

                let mut ca_stack = Stack::new().map_err(TlsError::NewCaStack)?;
                for intermediate in crt_stack {
                    ca_stack.push(intermediate).map_err(TlsError::CaStackPush)?;
                }

                let mut builder = Pkcs12::builder();
                builder.ca(ca_stack);
                let pkcs12 = builder
                    .build("", &name, &key, &crt)
                    .map_err(TlsError::BuildPkcs12)?;
                let identity = pkcs12.to_der().map_err(TlsError::DerExport)?;

                // Build the resulting parsed PKCS#12 archive,
                // but don't store it, as it cannot be cloned.
                // This is just for error checking.
                pkcs12.parse("").map_err(TlsError::Identity)?;

                Ok(Some(IdentityStore(identity, "".into())))
            }
        }
    }

    /// Parse identity from a DER encoded PKCS#12 archive
    fn parse_pkcs12_identity(&self, der: Vec<u8>) -> Result<Option<IdentityStore>> {
        let pkcs12 = Pkcs12::from_der(&der).map_err(TlsError::ParsePkcs12)?;
        // Verify password
        let key_pass = self.key_pass.as_deref().unwrap_or("");
        pkcs12.parse(key_pass).map_err(TlsError::ParsePkcs12)?;
        Ok(Some(IdentityStore(der, key_pass.to_string())))
    }
}

/// === System Specific Root Cert ===
///
/// Most of this code is borrowed from https://github.com/ctz/rustls-native-certs

/// Load the system default certs from `schannel` this should be in place
/// of openssl-probe on linux.
#[cfg(windows)]
fn load_windows_certs(builder: &mut SslContextBuilder) -> Result<()> {
    use super::Schannel;

    let mut store = X509StoreBuilder::new().context(NewStoreBuilder)?;

    let current_user_store =
        schannel::cert_store::CertStore::open_current_user("ROOT").context(Schannel)?;

    for cert in current_user_store.certs() {
        let cert = cert.to_der().to_vec();
        let cert = X509::from_der(&cert[..]).context(super::X509SystemParse)?;
        store.add_cert(cert).context(AddCertToStore)?;
    }

    builder
        .set_verify_cert_store(store.build())
        .context(SetVerifyCert)?;

    Ok(())
}

#[cfg(target_os = "macos")]
fn load_mac_certs(builder: &mut SslContextBuilder) -> Result<()> {
    use std::collections::HashMap;

    use security_framework::trust_settings::{Domain, TrustSettings, TrustSettingsForCertificate};

    use super::SecurityFramework;

    // The various domains are designed to interact like this:
    //
    // "Per-user Trust Settings override locally administered
    //  Trust Settings, which in turn override the System Trust
    //  Settings."
    //
    // So we collect the certificates in this order; as a map of
    // their DER encoding to what we'll do with them.  We don't
    // overwrite existing elements, which mean User settings
    // trump Admin trump System, as desired.

    let mut store = X509StoreBuilder::new().context(NewStoreBuilder)?;
    let mut all_certs = HashMap::new();

    for domain in &[Domain::User, Domain::Admin, Domain::System] {
        let ts = TrustSettings::new(*domain);

        for cert in ts.iter().context(SecurityFramework)? {
            // If there are no specific trust settings, the default
            // is to trust the certificate as a root cert.  Weird API but OK.
            // The docs say:
            //
            // "Note that an empty Trust Settings array means "always trust this cert,
            //  with a resulting kSecTrustSettingsResult of kSecTrustSettingsResultTrustRoot".
            let trusted = ts
                .tls_trust_settings_for_certificate(&cert)
                .context(SecurityFramework)?
                .unwrap_or(TrustSettingsForCertificate::TrustRoot);

            all_certs.entry(cert.to_der()).or_insert(trusted);
        }
    }

    for (cert, trusted) in all_certs {
        if matches!(
            trusted,
            TrustSettingsForCertificate::TrustRoot | TrustSettingsForCertificate::TrustAsRoot
        ) {
            let cert = X509::from_der(&cert[..]).context(super::X509SystemParse)?;
            store.add_cert(cert).context(AddCertToStore)?;
        }
    }

    builder
        .set_verify_cert_store(store.build())
        .context(SetVerifyCert)?;

    Ok(())
}

impl fmt::Debug for TlsSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TlsSettings")
            .field("verify_certificate", &self.verify_certificate)
            .field("verify_hostname", &self.verify_hostname)
            .finish()
    }
}

pub type MaybeTlsSettings = MaybeTls<(), TlsSettings>;

impl MaybeTlsSettings {
    pub fn enable_client() -> Result<Self> {
        let tls = TlsSettings::from_options_base(&None, false)?;
        Ok(Self::Tls(tls))
    }

    /// Generate an optional settings struct from the given optional
    /// configuration reference. If `config` is `None`, TLS is
    /// disabled. The `for_server` parameter indicates the options
    /// should be interpreted as being for a TLS server, which requires
    /// an identity certificate and changes the certificate verification
    /// default to false.
    pub fn from_config(config: &Option<TlsConfig>, for_server: bool) -> Result<Self> {
        match config {
            None => Ok(Self::Raw(())), // No config, no TLS settings
            Some(config) => {
                let tls = TlsSettings::from_options_base(&Some(config.clone()), for_server)?;
                match (for_server, &tls.identity) {
                    // Servers require an identity certificate
                    (true, None) => Err(TlsError::MissingRequiredIdentity),
                    _ => Ok(Self::Tls(tls)),
                }
            }
        }
    }

    pub const fn http_protocol_name(&self) -> &'static str {
        match self {
            MaybeTls::Raw(_) => "http",
            MaybeTls::Tls(_) => "https",
        }
    }

    pub fn client_config(config: &Option<TlsConfig>) -> Result<Self> {
        Self::from_config(config, false)
    }
}

impl From<TlsSettings> for MaybeTlsSettings {
    fn from(tls: TlsSettings) -> Self {
        Self::Tls(tls)
    }
}

/// Load a private key from a named file
fn load_key(filename: &Path, pass_phrase: &Option<String>) -> Result<PKey<Private>> {
    let (data, filename) = open_read(filename, "key")?;
    match pass_phrase {
        None => der_or_pem(
            data,
            |der| PKey::private_key_from_der(&der),
            |pem| PKey::private_key_from_pem(pem.as_bytes()),
        )
        .map_err(|err| TlsError::PrivateKeyParse { filename, err }),
        Some(phrase) => der_or_pem(
            data,
            |der| PKey::private_key_from_pkcs8_passphrase(&der, phrase.as_bytes()),
            |pem| PKey::private_key_from_pem_passphrase(pem.as_bytes(), phrase.as_bytes()),
        )
        .map_err(|err| TlsError::PrivateKeyParse { filename, err }),
    }
}

/// Parse the data one way if it looks like a DER file, and the other if
/// it looks like a PEM file. For the content to be treated as PEM, it
/// must parse as valid UTF-8 and contain a PEM start marker.
fn der_or_pem<T>(data: Vec<u8>, der_fn: impl Fn(Vec<u8>) -> T, pem_fn: impl Fn(String) -> T) -> T {
    // None of these steps cause (re)allocations,
    // just parsing and type manipulation
    match String::from_utf8(data) {
        Ok(text) => match text.find(PEM_START_MARKER) {
            Some(_) => pem_fn(text),
            None => der_fn(text.into_bytes()),
        },
        Err(err) => der_fn(err.into_bytes()),
    }
}

/// Open the named file and read its entire contents into memory. If the
/// file "name" contains a PEM start marker, it is assumed to contain
/// inline data and is used directly instead of opening a file.
fn open_read(filename: &Path, note: &'static str) -> Result<(Vec<u8>, PathBuf)> {
    if let Some(filename) = filename.to_str() {
        if filename.contains(PEM_START_MARKER) {
            return Ok((Vec::from(filename), "inline text".into()));
        }
    }

    let mut text = Vec::<u8>::new();

    File::open(filename)
        .map_err(|err| TlsError::FileOpenFailed {
            note,
            filename: filename.into(),
            err,
        })?
        .read_to_end(&mut text)
        .map_err(|err| TlsError::FileReadFailed {
            note,
            filename: filename.into(),
            err,
        })?;

    Ok((text, filename.into()))
}

#[cfg(test)]
mod test {
    use super::*;

    const TEST_PKCS12_PATH: &str = "tests/fixtures/tls/localhost.p12";
    const TEST_PEM_CRT_BYTES: &[u8] = include_bytes!("../../tests/fixtures/tls/localhost.crt");
    const TEST_PEM_KEY_BYTES: &[u8] = include_bytes!("../../tests/fixtures/tls/localhost.key");

    #[test]
    fn from_options_pkcs12() {
        let options = TlsConfig {
            crt_file: Some(TEST_PKCS12_PATH.into()),
            key_pass: Some("NOPASS".into()),
            ..Default::default()
        };
        let settings =
            TlsSettings::from_options(&Some(options)).expect("Failed to load PKCS#12 certificate");
        assert!(settings.identity.is_some());
        assert_eq!(settings.authorities.len(), 0);
    }

    #[test]
    fn from_options_pem() {
        let options = TlsConfig {
            crt_file: Some(TEST_PEM_CRT_PATH.into()),
            key_file: Some(TEST_PEM_KEY_PATH.into()),
            ..Default::default()
        };
        let settings =
            TlsSettings::from_options(&Some(options)).expect("Failed to load PEM certificate");
        assert!(settings.identity.is_some());
        assert_eq!(settings.authorities.len(), 0);
    }

    #[test]
    fn from_options_inline_pem() {
        let crt = String::from_utf8(TEST_PEM_CRT_BYTES.to_vec()).unwrap();
        let key = String::from_utf8(TEST_PEM_KEY_BYTES.to_vec()).unwrap();
        let options = TlsConfig {
            crt_file: Some(crt.into()),
            key_file: Some(key.into()),
            ..Default::default()
        };
        let settings =
            TlsSettings::from_options(&Some(options)).expect("Failed to load PEM certificate");
        assert!(settings.identity.is_some());
        assert_eq!(settings.authorities.len(), 0);
    }

    #[test]
    fn from_options_ca() {
        let options = TlsConfig {
            ca_file: Some(TEST_PEM_CA_PATH.into()),
            ..Default::default()
        };
        let settings = TlsSettings::from_options(&Some(options))
            .expect("Failed to load authority certificate");
        assert!(settings.identity.is_none());
        assert_eq!(settings.authorities.len(), 1);
    }

    #[test]
    fn from_options_inline_ca() {
        let ca =
            String::from_utf8(include_bytes!("../../tests/fixtures/tls/Vertex_CA.crt").to_vec())
                .unwrap();
        let options = TlsConfig {
            ca_file: Some(ca.into()),
            ..Default::default()
        };
        let settings = TlsSettings::from_options(&Some(options))
            .expect("Failed to load authority certificate");
        assert!(settings.identity.is_none());
        assert_eq!(settings.authorities.len(), 1);
    }

    #[test]
    fn from_options_intermediate_ca() {
        let options = TlsConfig {
            ca_file: Some("tests/fixtures/tls/Chain_with_intermediate.crt".into()),
            ..Default::default()
        };
        let settings = TlsSettings::from_options(&Some(options))
            .expect("Failed to load authority certificate");
        assert!(settings.identity.is_none());
        assert_eq!(settings.authorities.len(), 3);
    }

    #[test]
    fn from_options_multi_ca() {
        let options = TlsConfig {
            ca_file: Some("tests/fixtures/tls/Multi_CA.crt".into()),
            ..Default::default()
        };
        let settings = TlsSettings::from_options(&Some(options))
            .expect("Failed to load authority certificate");
        assert!(settings.identity.is_none());
        assert_eq!(settings.authorities.len(), 2);
    }

    #[test]
    fn from_options_none() {
        let settings = TlsSettings::from_options(&None).expect("Failed to generate null settings");
        assert!(settings.identity.is_none());
        assert_eq!(settings.authorities.len(), 0);
    }

    #[test]
    fn from_options_bad_certificate() {
        let options = TlsConfig {
            key_file: Some(TEST_PEM_KEY_PATH.into()),
            ..Default::default()
        };
        let error = TlsSettings::from_options(&Some(options))
            .expect_err("from_options failed to check certificate");
        assert!(matches!(error, TlsError::MissingCrtKeyFile));

        let options = TlsConfig {
            crt_file: Some(TEST_PEM_CRT_PATH.into()),
            ..Default::default()
        };
        let _error = TlsSettings::from_options(&Some(options))
            .expect_err("from_options failed to check certificate");
        // Actual error is an ASN parse, doesn't really matter
    }

    #[test]
    fn from_config_none() {
        assert!(MaybeTlsSettings::from_config(&None, true).unwrap().is_raw());
        assert!(MaybeTlsSettings::from_config(&None, false)
            .unwrap()
            .is_raw());
    }

    #[test]
    fn from_config_not_enabled() {
        assert!(settings_from_config(false, false, true).is_raw());
        assert!(settings_from_config(false, false, false).is_raw());
    }

    #[test]
    fn from_config_fails_without_certificate() {
        let config = make_config(false, false);
        let error = MaybeTlsSettings::from_config(&Some(config), true)
            .expect_err("from_config failed to check for a certificate");
        assert!(matches!(error, TlsError::MissingRequiredIdentity));
    }

    #[test]
    fn from_config_with_certificate() {
        let config = settings_from_config(true, true, true);
        assert!(config.is_tls());
    }

    fn settings_from_config(set_crt: bool, set_key: bool, for_server: bool) -> MaybeTlsSettings {
        let config = if !set_key && !set_crt {
            None
        } else {
            Some(make_config(set_crt, set_key))
        };

        MaybeTlsSettings::from_config(&config, for_server)
            .expect("Failed to generate settings from config")
    }

    fn make_config(set_crt: bool, set_key: bool) -> TlsConfig {
        TlsConfig {
            crt_file: set_crt.then(|| TEST_PEM_CRT_PATH.into()),
            key_file: set_key.then(|| TEST_PEM_KEY_PATH.into()),
            ..Default::default()
        }
    }
}
