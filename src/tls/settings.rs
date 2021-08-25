use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    fs::File,
    io::{
        self,
        BufReader,
    },
};
use hyper::{
    client::{
        Client, HttpConnector,
        connect::dns::GaiResolver,
    }
};
use hyper_rustls::HttpsConnector;
use hyper::client::connect::Connect;
use hyper::body::HttpBody;

const PEM_START_MARKER: &str = "-----BEGIN ";

#[cfg(test)]
pub const TEST_PEM_CA_PATH: &str = "tests/data/Vector_CA.crt";
#[cfg(test)]
pub const TEST_PEM_CRT_PATH: &str = "tests/data/localhost.crt";
#[cfg(test)]
pub const TEST_PEM_KEY_PATH: &str = "tests/data/localhost.key";

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TLSConfig {
    pub verify_certificate: Option<bool>,
    pub verify_hostname: Option<bool>,
    pub ca_file: Option<PathBuf>,
    pub crt_file: Option<PathBuf>,
    pub key_file: Option<PathBuf>,
    pub key_pass: Option<String>,
}

impl TLSConfig {
    #[cfg(test)]
    pub fn test_options() -> Self {
        Self {
            ca_file: Some(TEST_PEM_CA_PATH.into()),
            crt_file: Some(TEST_PEM_CRT_PATH.into()),
            key_file: Some(TEST_PEM_KEY_PATH.into()),
            ..Self::default()
        }
    }

    fn load_authorities(&self) {
        todo!()
    }

/*
    async fn hyper_client<C, B>(&self) -> Result<Client<C, B>, io::Error>
        where
            C: Clone + Connect,
            B: HttpBody + Send,
            B::Data: Send
    {
        let mut ca = match self.ca_file {
            Some(ref path) => {
                let f = File::open(path)
                    .map_err(|err| error(format!("failed to open {}: {}", path.to_str().unwrap(), err)))?;

                let rd = BufReader::new(f);
                Some(rd)
            }
            None => None,
        };

        let https = match ca {
            Some(ref mut rd) => {
                // Build an HTTP connector which supports HTTPS too.
                let mut http = HttpConnector::new();
                http.enforce_http(false);

                // Build a TLS client, using the custom CA store for lookups
                let mut tls = rustls::ClientConfig::new();
                tls.root_store
                    .add_pem_file(rd)
                    .map_err(|_| error("failed to load custom CA store".into()))?;
                // Join the above part into an HTTPS connector
                hyper_rustls::HttpsConnector::from((http, tls))
            }

            // Default HTTPS connector
            None => hyper_rustls::HttpsConnector::with_native_roots()
        };

        Ok(Client::builder().build(https))
    }
    */

}

fn error(err: String) -> io::Error {
    io::Error::new(io::ErrorKind::Other, err)
}
