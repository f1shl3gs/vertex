use bytes::{BufMut, BytesMut};

use super::auth::AuthPlugin;
use super::{
    CLIENT_PLUGIN_AUTH, Deserialize, Error, PLUGIN_AUTH_LENENC_DATA, SECURE_CONNECTION, Serialize,
};

/// https://mariadb.com/docs/server/reference/clientserver-protocol/1-connecting/connection#client-handshake-response#initial-handshake-packet
#[derive(Debug)]
#[allow(dead_code)]
pub struct Handshake<'a> {
    pub server_version: &'a str,
    connection_id: u32,
    capabilities: u32,
    collation: u8,
    status: u16,
    pub auth_plugin: Option<AuthPlugin>,
    pub auth_plugin_data: (&'a [u8], &'a [u8]),
}

impl<'a> Deserialize<'a> for Handshake<'a> {
    fn deserialize(buf: &'a [u8]) -> Result<Self, Error> {
        debug_assert!(buf.len() >= 32, "want 32 but got {}", buf.len());

        let mut pos = 0;
        let version = buf[pos];
        if version != 10 {
            return Err(Error::UnsupportedVersion(version));
        }

        while pos < buf.len() {
            if buf[pos] == 0 {
                break;
            }

            pos += 1;
        }
        if pos >= buf.len() {
            return Err(Error::Eof);
        }

        let server_version = &buf[1..pos];
        pos += 1; // skip the null terminator

        if pos + 4 > buf.len() {
            return Err(Error::Eof);
        }
        let mut connection_id = buf[pos] as u32;
        connection_id |= (buf[pos + 1] as u32) << 8;
        connection_id |= (buf[pos + 2] as u32) << 16;
        connection_id |= (buf[pos + 3] as u32) << 24;
        pos += 4;

        // string<8>
        if pos + 8 > buf.len() {
            return Err(Error::Eof);
        }
        let auth_plugin_data1 = &buf[pos..pos + 8];
        pos += 8;

        // skip reserved string<1>
        pos += 1;

        if pos + 2 > buf.len() {
            return Err(Error::Eof);
        }
        let cap_1 = buf[pos] as u16 | (buf[pos + 1] as u16) << 8;
        pos += 2;

        // int<1>
        if pos + 1 > buf.len() {
            return Err(Error::Eof);
        }
        let collation = buf[pos];
        pos += 1;

        // status: int<2>
        if pos + 2 > buf.len() {
            return Err(Error::Eof);
        }
        let status = buf[pos] as u16 | (buf[pos + 1] as u16) << 8;
        pos += 2;

        // int<2>
        if pos + 2 > buf.len() {
            return Err(Error::Eof);
        }
        let cap_2 = buf[pos] as u16 | (buf[pos + 1] as u16) << 8;
        pos += 2;

        let mut capabilities = ((cap_2 as u32) << 16) | (cap_1 as u32);

        let auth_plugin_data_len = if capabilities & CLIENT_PLUGIN_AUTH != 0 {
            buf[pos] as usize
        } else {
            0
        };
        pos += 1;

        // reserved
        pos += 6;

        // mysql
        if capabilities & 1 != 0 {
            // reserved string<4>
            pos += 4;
        } else {
            let mut cap_3 = buf[pos] as u32;
            cap_3 |= (buf[pos + 1] as u32) << 8;
            cap_3 |= (buf[pos + 2] as u32) << 16;
            cap_3 |= (buf[pos + 3] as u32) << 24;
            pos += 4;

            capabilities |= cap_3;
        }

        let auth_plugin_data2 = if capabilities & SECURE_CONNECTION != 0 {
            let len = std::cmp::max(auth_plugin_data_len.saturating_sub(9), 12);
            let data = &buf[pos..pos + len];

            // +1 for skipping the null terminator
            pos += len + 1;

            data
        } else {
            &buf[0..0]
        };

        // auth plugin data
        let auth_plugin = if capabilities & CLIENT_PLUGIN_AUTH != 0 {
            Some(AuthPlugin::try_from(&buf[pos..])?)
        } else {
            None
        };

        Ok(Self {
            server_version: unsafe { std::str::from_utf8_unchecked(server_version) },
            connection_id,
            capabilities,
            collation,
            status,

            auth_plugin,
            auth_plugin_data: (auth_plugin_data1, auth_plugin_data2),
        })
    }
}

/// https://dev.mysql.com/doc/internals/en/connection-phase-packets.html#packet-Protocol::HandshakeResponse
/// https://mariadb.com/kb/en/connection/#client-handshake-response
#[derive(Debug)]
pub struct HandshakeResponse<'a> {
    pub capabilities: u32,

    /// Max size of a command packet that the client wants to send to the server
    pub max_packet_size: u32,

    /// Default charset (collation ID < 256) for the connection
    pub charset: u8,

    /// Name of the SQL account which client wants to login
    pub username: &'a str,

    /// Authentication method used by the client
    pub auth_plugin: Option<AuthPlugin>,

    /// Opaque authentication response
    pub auth_resp: Option<Vec<u8>>,
}

impl<'a> Serialize for HandshakeResponse<'a> {
    fn serialize(&self, buf: &mut BytesMut) {
        // https://mariadb.com/docs/server/reference/clientserver-protocol/1-connecting/connection#handshake-response-packet

        // buf.resize(4 + 88, 0);

        buf.put_u32_le(self.capabilities);
        buf.put_u32_le(self.max_packet_size);
        buf.put_u8(self.charset);
        buf.put_slice(&[
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]);
        // buf.advance(19); // reserved
        // buf.advance(4); // we don't have CLIENT_MYSQL

        buf.put_slice(self.username.as_bytes());
        buf.put_u8(0); // null terminator

        // auth plugin data
        if let Some(data) = &self.auth_resp {
            let len = data.len();

            if self.capabilities & PLUGIN_AUTH_LENENC_DATA != 0 {
                // https://dev.mysql.com/doc/internals/en/integer.html
                // https://mariadb.com/kb/en/library/protocol-data-types/#length-encoded-integers

                let lb = len.to_le_bytes();

                match len {
                    0..=250 => buf.put_u8(lb[0]),
                    251..=0xFF_FF => {
                        buf.put_u8(0xfc);
                        buf.put_slice(&lb[..2]);
                    }
                    0x1_00_00..=0xFF_FF_FF => {
                        buf.put_u8(0xfd);
                        buf.put_slice(&lb[..3]);
                    }
                    _ => {
                        buf.put_u8(0xfe);
                        buf.put_slice(&lb);
                    }
                }

                buf.put_slice(data);
            } else if self.capabilities & SECURE_CONNECTION != 0 {
                buf.put_u8(len as u8); // take case of overflow
                buf.put_slice(data);
            } else {
                buf.put_u8(0);
            }
        } else {
            buf.put_u8(0);
        }

        // no password

        // no default database

        // we do have CLIENT_PLUGIN_AUTH
        if let Some(plugin) = &self.auth_plugin {
            buf.put_slice(plugin.name().as_bytes());
        }
        buf.put_u8(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handshake() {
        const HSP: &[u8] = b"\x0a5.5.5-10.0.17-MariaDB-log\x00\x0b\x00\
                             \x00\x00\x64\x76\x48\x40\x49\x2d\x43\x4a\x00\xff\xf7\x08\x02\x00\
                             \x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x2a\x34\x64\
                             \x7c\x63\x5a\x77\x6b\x34\x5e\x5d\x3a\x00";
        let handshake = Handshake::deserialize(HSP).unwrap();
        println!("{:?}", handshake);

        assert_eq!(handshake.connection_id, 0x0b);
        assert_eq!(handshake.capabilities, 0xf7ff);
        assert_eq!(handshake.collation, 8);
        assert_eq!(handshake.status, 2);

        const HSP_2: &[u8] = b"\x0a\x35\x2e\x36\x2e\x34\x2d\x6d\x37\x2d\x6c\x6f\
                               \x67\x00\x56\x0a\x00\x00\x52\x42\x33\x76\x7a\x26\x47\x72\x00\xff\
                               \xff\x08\x02\x00\x0f\xc0\x15\x00\x00\x00\x00\x00\x00\x00\x00\x00\
                               \x00\x2b\x79\x44\x26\x2f\x5a\x5a\x33\x30\x35\x5a\x47\x00\x6d\x79\
                               \x73\x71\x6c\x5f\x6e\x61\x74\x69\x76\x65\x5f\x70\x61\x73\x73\x77\
                               \x6f\x72\x64\x00";
        let handshake = Handshake::deserialize(HSP_2).unwrap();
        println!("{:?}", handshake);

        assert_eq!(handshake.connection_id, 0x0a56);
        assert_eq!(handshake.capabilities, 0xc00fffff);
        assert_eq!(handshake.collation, 8);
        assert_eq!(handshake.status, 2);

        const HSP_3: &[u8] = b"\x0a\x35\x2e\x36\x2e\x34\x2d\x6d\x37\x2d\x6c\x6f\
                                \x67\x00\x56\x0a\x00\x00\x52\x42\x33\x76\x7a\x26\x47\x72\x00\xff\
                                \xff\x08\x02\x00\x0f\xc0\x15\x00\x00\x00\x00\x00\x00\x00\x00\x00\
                                \x00\x2b\x79\x44\x26\x2f\x5a\x5a\x33\x30\x35\x5a\x47\x00\x6d\x79\
                                \x73\x71\x6c\x5f\x6e\x61\x74\x69\x76\x65\x5f\x70\x61\x73\x73\x77\
                                \x6f\x72\x64\x00";
        let handshake = Handshake::deserialize(HSP_3).unwrap();
        println!("{:?}", handshake);

        assert_eq!(handshake.connection_id, 0x0a56);
        assert_eq!(handshake.capabilities, 0xc00fffff);
        assert_eq!(handshake.collation, 8);
        assert_eq!(handshake.status, 2);
    }

    const SERVER_STATUS_AUTOCOMMIT: u16 = 2;

    #[test]
    fn mariadb_10_4_7() {
        let input = b"\n5.5.5-10.4.7-MariaDB-1:10.4.7+maria~bionic\x00\x0b\x00\x00\x00t6L\\j\"dS\x00\xfe\xf7\x08\x02\x00\xff\x81\x15\x00\x00\x00\x00\x00\x00\x07\x00\x00\x00U14Oph9\"<H5n\x00mysql_native_password\x00";
        let handshake = Handshake::deserialize(input).unwrap();

        assert_eq!(handshake.collation, 0x08);
        assert_ne!(handshake.status & SERVER_STATUS_AUTOCOMMIT, 0);
        assert_eq!(handshake.auth_plugin, Some(AuthPlugin::Native))
    }

    #[test]
    fn mysql_8_0_18() {
        let input = b"\n8.0.18\x00\x19\x00\x00\x00\x114aB0c\x06g\x00\xff\xff\xff\x02\x00\xff\xc7\x15\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00tL\x03s\x0f[4\rl4. \x00caching_sha2_password\x00";
        let handshake = Handshake::deserialize(input).unwrap();

        assert_eq!(handshake.collation, 255);
        assert_ne!(handshake.status & SERVER_STATUS_AUTOCOMMIT, 0);
        assert_eq!(handshake.auth_plugin, Some(AuthPlugin::CachingSha2));
        assert_eq!(
            handshake.auth_plugin_data,
            (
                [17u8, 52, 97, 66, 48, 99, 6, 103].as_ref(),
                [116u8, 76, 3, 115, 15, 91, 52, 13, 108, 52, 46, 32].as_ref(),
            )
        );
    }
}
