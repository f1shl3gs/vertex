mod auth;
mod crypto;
mod handshake;
mod packet;
mod row;
mod version;

#[cfg(test)]
mod mock;

use std::net::SocketAddr;
use std::num::{ParseFloatError, ParseIntError};

use bytes::{BufMut, BytesMut};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use auth::xor_eq;
use handshake::{Handshake, HandshakeResponse};
use packet::{AuthSwitchRequest, ErrorPacket, OkPacket, QueryPacket, QuitPacket, get_lenenc};

pub use auth::{AuthConfig, AuthPlugin};
pub use row::{ColumnDefinition, Rows};
pub use version::{Flavor, Version};

#[cfg(test)]
pub use mock::mock;

const CHARSET_UTF8MB4: u8 = 45;

const CLIENT_PLUGIN_AUTH: u32 = 1 << 19;

// 4.1+ authentication
const SECURE_CONNECTION: u32 = 1 << 15;
// Enable authentication response packet to be larger than 255 bytes.
const PLUGIN_AUTH_LENENC_DATA: u32 = 1 << 21;

// Capabilities this client have, be aware SSL is not support yet
//
// PROTOCOL_41
// IGNORE_SPACE
// DEPRECATE_EOF
// FOUND_ROWS
// TRANSACTIONS
// SECURE_CONNECTION
// PLUGIN_AUTH_LENENC_DATA
// MULTI_STATEMENTS
// MULTI_RESULTS
// PLUGIN_AUTH
// PS_MULTI_RESULTS
const DEFAULT_CLIENT_CAPABILITIES: u32 = 19899138;

trait Deserialize<'de>: Sized {
    fn deserialize(buf: &'de [u8]) -> Result<Self, Error>;
}

trait Serialize {
    fn serialize(&self, buf: &mut BytesMut);
}

impl Serialize for &[u8] {
    fn serialize(&self, buf: &mut BytesMut) {
        buf.put_slice(self)
    }
}

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),

    NoData,

    UnsupportedAuthPlugin(String),

    UnsupportedVersion(u8),

    Eof,

    PasswordRequired,

    InvalidLengthEncoded,

    #[allow(clippy::upper_case_acronyms)]
    RSA(crypto::Error),

    Protocol(String),

    InvalidColumnType(&'static str),

    Server {
        code: u16,
        state: Option<String>,
        message: String,
    },
}

impl From<ErrorPacket<'_>> for Error {
    fn from(err: ErrorPacket<'_>) -> Self {
        Error::Server {
            code: err.code,
            state: err.state.map(|s| String::from_utf8_lossy(s).to_string()),
            message: String::from_utf8_lossy(err.message).to_string(),
        }
    }
}

impl From<crypto::Error> for Error {
    fn from(err: crypto::Error) -> Self {
        Error::RSA(err)
    }
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(err) => err.fmt(f),
            Error::NoData => f.write_str("no rows"),
            Error::PasswordRequired => f.write_str("password required"),
            Error::InvalidColumnType(want) => {
                f.write_fmt(format_args!("invalid data type found, expect {want}"))
            }
            Error::UnsupportedAuthPlugin(name) => {
                f.write_fmt(format_args!("unsupported auth plugin: {}", name))
            }
            Error::UnsupportedVersion(v) => {
                f.write_fmt(format_args!("unsupported mysql version: {}", v))
            }
            Error::Eof => f.write_str("EOF"),
            Error::Protocol(s) => f.write_fmt(format_args!("protocol: {}", s)),
            Error::Server {
                code,
                state,
                message,
            } => {
                if let Some(state) = state {
                    f.write_fmt(format_args!("{} ({}): {}", code, state, message))
                } else {
                    f.write_fmt(format_args!("{} ({})", code, message))
                }
            }
            Error::InvalidLengthEncoded => f.write_str("invalid length-encoded integer value"),
            Error::RSA(err) => err.fmt(f),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<ParseIntError> for Error {
    fn from(_err: ParseIntError) -> Self {
        Error::InvalidColumnType("INT")
    }
}

impl From<ParseFloatError> for Error {
    fn from(_err: ParseFloatError) -> Self {
        Error::InvalidColumnType("DECIMAL")
    }
}

const INITIAL_BUFFER_CAPACITY: usize = 512;

pub struct Connection {
    stream: TcpStream,
    buf: BytesMut,

    sequence: u8,
    version: Version,
}

impl Connection {
    pub async fn connect(endpoint: SocketAddr, auth: &AuthConfig) -> Result<Self, Error> {
        let conn = TcpStream::connect(endpoint).await?;

        conn.set_nodelay(true)?;

        let mut conn = Self {
            stream: conn,
            sequence: 0,
            version: Version::from((0, 0, 0)),

            buf: BytesMut::with_capacity(INITIAL_BUFFER_CAPACITY),
        };

        let packet = conn.recv_packet().await?;
        let handshake = Handshake::deserialize(packet)?;
        let version = Version::try_from(handshake.server_version)?;

        debug!(
            message = "receiving handshake request",
            version = handshake.server_version,
            auth = ?handshake.auth_plugin,
        );

        let auth_resp = match (&handshake.auth_plugin, &auth.password) {
            (Some(auth_plugin), Some(password)) => {
                Some(auth_plugin.scramble(password, handshake.auth_plugin_data))
            }
            _ => None,
        };

        let mut auth_nonce = Vec::with_capacity(
            handshake.auth_plugin_data.0.len() + handshake.auth_plugin_data.1.len(),
        );
        auth_nonce.extend_from_slice(handshake.auth_plugin_data.0);
        auth_nonce.extend_from_slice(handshake.auth_plugin_data.1);

        let mut auth_plugin = handshake.auth_plugin.clone();
        conn.send(HandshakeResponse {
            capabilities: DEFAULT_CLIENT_CAPABILITIES,
            max_packet_size: 16 * 1024,
            charset: CHARSET_UTF8MB4,
            username: &auth.username,
            auth_plugin: auth_plugin.clone(),
            auth_resp,
        })
        .await?;

        // we might have to handle auth data so a loop is necessary
        loop {
            let packet = conn.recv_packet().await?;
            if packet.len() < 2 {
                return Err(Error::Eof);
            }

            match packet[0] {
                // Auth OK
                0x00 => {
                    debug!(message = "handshake finished", auth = ?auth_plugin);

                    // ok packet
                    let _ok = OkPacket::deserialize(packet)?;
                    break;
                }

                // Auth more data
                0x01 => {
                    match packet[1] {
                        // fast auth success
                        0x03 => {
                            debug!(message = "fast auth success", auth = ?auth_plugin);

                            // auth ok
                            let packet = conn.recv_packet().await?;
                            let _ok = OkPacket::deserialize(packet)?;

                            break;
                        }
                        // perform full authentication
                        0x04 => {
                            debug!(message = "request for RSA public key", auth = ?auth_plugin);

                            // sends a public key request
                            conn.send([0x02].as_ref()).await?;
                            continue;
                        }
                        v => {
                            if let Some(password) = &auth.password
                                && matches!(
                                    &auth_plugin,
                                    Some(AuthPlugin::Sha256) | Some(AuthPlugin::CachingSha2)
                                )
                            {
                                debug!(message = "performing full authentication", auth = ?auth_plugin);

                                let public_key = &packet[1..];

                                // xor the password with the given nonce
                                let mut password = password.as_bytes().to_vec();
                                password.push(0);

                                xor_eq(&mut password, &auth_nonce);

                                let payload = crypto::encrypt(&password, public_key)?;

                                // sends an RSA encrypted password
                                conn.send(payload.as_ref()).await?;

                                continue;
                            }

                            return Err(Error::Protocol(format!(
                                "unexpected 0x{:x} when authenticating with 0x03 (fast_auth_success) or 0x04 (perform_full_authentication)",
                                v
                            )));
                        }
                    }
                }
                // auth switch
                0xfe => {
                    let auth_switch = AuthSwitchRequest::deserialize(packet)?;

                    debug!(
                        message = "switching authorization",
                        initial = ?auth_plugin,
                        to = ?auth_switch.auth_plugin
                    );

                    auth_plugin = Some(auth_switch.auth_plugin.clone());
                    auth_nonce.clear();
                    auth_nonce.extend_from_slice(auth_switch.data);

                    // for non-TLS connection only
                    if auth_switch.auth_plugin == AuthPlugin::Sha256 {
                        conn.send([0x01].as_ref()).await?;
                        continue;
                    }

                    let password = auth.password.as_ref().ok_or(Error::PasswordRequired)?;
                    let payload = auth_switch
                        .auth_plugin
                        .scramble(password, (auth_switch.data, &[]));

                    conn.send(payload.as_ref()).await?;
                }
                id => {
                    return Err(Error::Protocol(format!(
                        "unexpected packet 0x{:02x} for auth plugin {:?} during authentication",
                        id, auth_plugin
                    )));
                }
            }
        }

        conn.version = version;

        Ok(conn)
    }

    pub fn version(&self) -> Version {
        self.version.clone()
    }

    pub async fn query<S>(&mut self, query: S) -> Result<Rows<'_>, Error>
    where
        S: AsRef<str>,
    {
        self.sequence = 0;
        self.send(QueryPacket(query.as_ref())).await?;

        // read the column meta
        let packet = self.recv_packet().await?;

        // first packet in a query response is OK or ERROR
        if packet[0] == 0x00 {
            let _ok = OkPacket::deserialize(packet)?;

            // query shell not got this

            return Err(Error::Protocol(
                "OkPacket shell not received when query".to_string(),
            ));
        } else if packet[0] == 0xff {
            // err
            let err = ErrorPacket::deserialize(packet)?;
            return Err(err.into());
        }

        // otherwise, this first packet is the start of the result-set metadata
        let size = get_lenenc(packet, &mut 0)? as usize;
        let mut columns = Vec::with_capacity(size);
        for _ in 0..size {
            let packet = self.recv_packet().await?;
            columns.push(ColumnDefinition::deserialize(packet)?);
        }

        Ok(Rows {
            columns,
            stream: &mut self.stream,
            buf: &mut self.buf,
        })
    }

    /// Explicitly close this database connection
    pub async fn close(mut self) -> Result<(), Error> {
        self.send(QuitPacket).await?;
        self.stream.shutdown().await?;
        Ok(())
    }

    async fn send<T: Serialize>(&mut self, req: T) -> Result<(), Error> {
        self.buf.resize(4, 0); // placeholder for packet header

        req.serialize(&mut self.buf);

        let mut header: [u8; 4] = (self.buf.len() as u32 - 4).to_le_bytes();
        header[3] = self.sequence;

        self.buf[..4].copy_from_slice(&header);

        self.stream.write_all(&self.buf).await?;

        Ok(())
    }

    // TODO: try receive as many as possible, and parsing when needed
    async fn recv_packet(&mut self) -> Result<&[u8], Error> {
        let mut header = [0u8; 4];

        self.stream.read_exact(&mut header).await?;

        let mut len = header[0] as u32;
        len |= (header[1] as u32) << 8;
        len |= (header[2] as u32) << 16;
        self.sequence = header[3].wrapping_add(1);

        self.buf.resize(len as usize, 0);
        self.stream.read_exact(self.buf.as_mut()).await?;

        let buf = self.buf.as_ref();
        if let Some(0xff) = buf.first() {
            let err = ErrorPacket::deserialize(buf)?;

            return Err(err.into());
        }

        Ok(buf)
    }
}

#[cfg(test)]
impl Connection {
    pub fn set_flavor(&mut self, flavor: Flavor) {
        self.version.set_flavor(flavor);
    }

    pub fn set_version(&mut self, major: u8, minor: u8, patch: u8) {
        self.version.set(major, minor, patch);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;

    #[tokio::test]
    async fn connect() {
        let endpoint = SocketAddr::new(IpAddr::from([127, 0, 0, 1]), 9150);
        let auth = AuthConfig {
            username: "root".to_string(),
            password: Some("password".to_string()),
        };

        let mut conn = Connection::connect(endpoint, &auth).await.unwrap();

        let mut rows = conn.query("select @@version").await.unwrap();
        while let Some(mut row) = rows.next().await.unwrap() {
            for column in row.columns() {
                println!("{} {}", column.name(), row.get_str());
            }
        }

        conn.close().await.unwrap();
    }

    #[test]
    fn version() {
        let version = Version::from((0, 0, 9));
        assert_eq!(version, 0.0);
        assert!(version < 0.1);

        let version = Version::from((1, 0, 0));
        assert_eq!(version, 1.0);
        assert!(version > 0.9);
        assert!(version < 1.1);
    }
}

/*
#[cfg(all(test, feature = "mysql-integration-tests"))]
mod integration_tests {
    use super::*;
    use std::str::FromStr;

    #[tokio::test]
    async fn multi_password() {
        for username in ["user_native", "user_caching", "user_sha256", "user_clear"] {
            let auth = AuthConfig {
                username: username.to_string(),
                password: Some("password".to_string()),
            };

            let addr = SocketAddr::from_str("127.0.0.1:4407").unwrap();
            let mut conn = Connection::connect(addr, &auth).await.unwrap();

            let mut rows = conn.query("select @@version").await.unwrap();
            while let Some(mut row) = rows.next().await.unwrap() {
                for column in row.columns() {
                    println!("{} {}", column.name(), row.get_str());
                }
            }

            conn.close().await.unwrap();
        }
    }
}
*/
