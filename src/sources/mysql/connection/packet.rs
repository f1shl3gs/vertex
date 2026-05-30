use bytes::{BufMut, BytesMut};

use super::auth::AuthPlugin;
use super::{Deserialize, Error, Serialize};

// https://dev.mysql.com/doc/dev/mysql-server/8.0.12/page_protocol_connection_phase_packets_protocol_auth_switch_request.html
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug)]
pub struct AuthSwitchRequest<'a> {
    pub auth_plugin: AuthPlugin,
    pub data: &'a [u8],
}

impl<'de> Deserialize<'de> for AuthSwitchRequest<'de> {
    fn deserialize(buf: &'de [u8]) -> Result<Self, Error> {
        let header = buf[0];
        if header != 0xfe {
            return Err(Error::Protocol(format!(
                "expected 0xfe (AUTH_SWITCH) but found 0x{:x}",
                header
            )));
        }

        let mut pos = 1;
        while pos < buf.len() {
            if buf[pos] == 0 {
                break;
            }

            pos += 1;
        }
        if pos >= buf.len() {
            return Err(Error::Eof);
        }

        let plugin_name = &buf[1..pos];
        pos += 1;

        let auth_plugin = match plugin_name {
            b"mysql_native_password" => AuthPlugin::Native,
            b"caching_sha2_password" => AuthPlugin::CachingSha2,
            b"sha256_password" => AuthPlugin::Sha256,
            b"mysql_clear_password" => AuthPlugin::Clear,
            _ => {
                return Err(Error::UnsupportedAuthPlugin(
                    String::from_utf8_lossy(plugin_name).to_string(),
                ));
            }
        };

        if buf.len() == pos {
            return Ok(Self {
                auth_plugin,
                data: &[],
            });
        }

        // See: https://github.com/mysql/mysql-server/blob/ea7d2e2d16ac03afdd9cb72a972a95981107bf51/sql/auth/sha2_password.cc#L942
        if buf.len() - pos != 21 {
            return Err(Error::Protocol(format!(
                "expected 21 bytes but got {} bytes",
                buf.len() - pos
            )));
        }

        let data = &buf[pos..pos + 20];

        Ok(AuthSwitchRequest { auth_plugin, data })
    }
}

pub struct QueryPacket<'a>(pub &'a str);

impl<'a> Serialize for QueryPacket<'a> {
    fn serialize(&self, buf: &mut BytesMut) {
        // Text format
        //
        // https://dev.mysql.com/doc/internals/en/com-query.html

        // COM_QUERY
        buf.put_u8(0x03);
        buf.put_slice(self.0.as_bytes());
    }
}

/// Indicates successful completion of a previous command sent by the client.
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug)]
pub struct OkPacket {
    pub affected_rows: u64,
    pub last_insert_id: u64,
    pub status: u16,
    #[allow(dead_code)]
    pub warnings: u16,
}

impl<'de> Deserialize<'de> for OkPacket {
    fn deserialize(buf: &'de [u8]) -> Result<Self, Error> {
        let header = buf[0];
        if header != 0x00 && header != 0xfe {
            return Err(Error::Protocol(format!(
                "expected 0x00 or 0xfe (Ok_Packet) but found 0x{:02x}",
                header
            )));
        }

        let mut pos = 1;
        let affected_rows = get_lenenc(buf, &mut pos)?;
        let last_insert_id = get_lenenc(buf, &mut pos)?;

        if buf.len() - pos < 4 {
            return Err(Error::Eof);
        }
        let status = buf[pos] as u16 | (buf[pos + 1] as u16) << 8;
        let warnings = buf[pos + 2] as u16 | (buf[pos + 3] as u16) << 8;

        Ok(Self {
            affected_rows,
            last_insert_id,
            status,
            warnings,
        })
    }
}

/// Indicates that an error occurred.
///
/// https://dev.mysql.com/doc/dev/mysql-server/8.0.12/page_protocol_basic_err_packet.html
/// https://mariadb.com/kb/en/err_packet/
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug)]
pub struct ErrorPacket<'a> {
    pub code: u16,
    pub state: Option<&'a [u8]>,
    pub message: &'a [u8],
}

impl<'de> Deserialize<'de> for ErrorPacket<'de> {
    fn deserialize(buf: &'de [u8]) -> Result<Self, Error> {
        if buf[0] != 0xff {
            return Err(Error::Protocol(format!(
                "invalid protocol byte {:x}",
                buf[0]
            )));
        }

        let code = buf[1] as u16 | ((buf[2] as u16) << 8);

        let (pos, state) = if buf[3] == b'#' {
            (9, Some(&buf[4..9]))
        } else {
            (3, None)
        };

        Ok(Self {
            code,
            state,
            message: &buf[pos..],
        })
    }
}

/// Marks the end of a result set, returning status and warnings
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug)]
pub struct EofPacket {
    #[allow(dead_code)]
    pub warnings: u16,
    pub status: u16,
}

impl<'de> Deserialize<'de> for EofPacket {
    fn deserialize(buf: &'de [u8]) -> Result<Self, Error> {
        if buf.len() < 5 {
            return Err(Error::Eof);
        }

        let header = buf[0];
        if header != 0xfe {
            return Err(Error::Protocol(format!(
                "expected 0xfe (EOF_Packet) but found 0x{:x}",
                header
            )));
        }

        let warnings = buf[1] as u16 | (buf[2] as u16) << 8;
        let status = buf[3] as u16 | (buf[4] as u16) << 8;

        Ok(Self { warnings, status })
    }
}

// https://dev.mysql.com/doc/internals/en/com-quit.html
pub struct QuitPacket;

impl Serialize for QuitPacket {
    fn serialize(&self, buf: &mut BytesMut) {
        buf.put_u8(0x01);
    }
}

pub fn get_lenenc(buf: &[u8], pos: &mut usize) -> Result<u64, Error> {
    if *pos >= buf.len() {
        return Err(Error::Eof);
    }

    let first = buf[*pos];
    *pos += 1;

    match first {
        value if value <= 0xfa => Ok(value as u64),
        0xfc => {
            if *pos + 2 > buf.len() {
                return Err(Error::Eof);
            }

            let mut v = buf[*pos] as u64;
            v |= (buf[*pos + 1] as u64) << 8;
            *pos += 2;

            Ok(v)
        }
        0xfd => {
            if *pos + 3 > buf.len() {
                return Err(Error::Eof);
            }

            let mut v = buf[*pos] as u64;
            v |= (buf[*pos + 1] as u64) << 8;
            v |= (buf[*pos + 2] as u64) << 16;
            *pos += 3;

            Ok(v)
        }
        0xfe => {
            if *pos + 8 > buf.len() {
                return Err(Error::Eof);
            }

            let mut v = buf[*pos] as u64;
            v |= (buf[*pos + 1] as u64) << 8;
            v |= (buf[*pos + 2] as u64) << (2 * 8);
            v |= (buf[*pos + 3] as u64) << (3 * 8);
            v |= (buf[*pos + 4] as u64) << (4 * 8);
            v |= (buf[*pos + 5] as u64) << (5 * 8);
            v |= (buf[*pos + 6] as u64) << (6 * 8);
            v |= (buf[*pos + 7] as u64) << (7 * 8);
            *pos += 8;

            Ok(v)
        }
        0xfb | 0xff => Err(Error::InvalidLengthEncoded),
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use super::*;

    const SERVER_STATUS_AUTOCOMMIT: u16 = 2;
    const SERVER_SESSION_STATE_CHANGED: u16 = 1 << 14;

    #[test]
    fn ok() {
        for (input, want) in [
            (
                b"\x00\x00\x00\x02@\x00\x00".as_ref(),
                OkPacket {
                    affected_rows: 0,
                    last_insert_id: 0,
                    warnings: 0,
                    status: SERVER_STATUS_AUTOCOMMIT | SERVER_SESSION_STATE_CHANGED,
                },
            ),
            (
                b"\xfe\x01\x00\x02\x00\x00\x00\x05\x09info data".as_ref(),
                OkPacket {
                    affected_rows: 1,
                    last_insert_id: 0,
                    warnings: 0,
                    status: SERVER_STATUS_AUTOCOMMIT,
                },
            ),
            (
                b"\xfe\x05\x64\x02\x00\x01\x00\x0e\x14extended information",
                OkPacket {
                    affected_rows: 5,
                    last_insert_id: 100,
                    warnings: 1,
                    status: SERVER_STATUS_AUTOCOMMIT,
                },
            ),
        ] {
            let got = OkPacket::deserialize(input).unwrap();
            assert_eq!(got, want);
        }

        assert_matches!(OkPacket::deserialize(b"\x00\x00\x00\x01"), Err(Error::Eof));
    }

    #[test]
    fn eof() {
        let input = b"\xfe\x00\x00\x02\x00";
        let got = EofPacket::deserialize(input).unwrap();
        assert_eq!(
            got,
            EofPacket {
                warnings: 0,
                status: SERVER_STATUS_AUTOCOMMIT,
            }
        );
    }

    #[test]
    fn error() {
        for (input, want) in [
            (
                b"\xff\x84\x04Got packets out of order".as_ref(),
                ErrorPacket {
                    code: 1156,
                    state: None,
                    message: b"Got packets out of order",
                },
            ),
            (
                b"\xff\x19\x04#42000Unknown database \'unknown\'",
                ErrorPacket {
                    code: 1049,
                    state: Some(b"42000"),
                    message: b"Unknown database \'unknown\'",
                },
            ),
        ] {
            let got = ErrorPacket::deserialize(input).unwrap();
            assert_eq!(got, want);
        }
    }

    #[test]
    fn auth_switch_request() {
        for (input, want) in [
            (
                b"\xfecaching_sha2_password\x00abcdefghijabcdefghij\x00".as_ref(),
                AuthSwitchRequest {
                    auth_plugin: AuthPlugin::CachingSha2,
                    data: b"abcdefghijabcdefghij",
                },
            ),
            (
                b"\xfemysql_clear_password\x00abcdefghijabcdefghij\x00",
                AuthSwitchRequest {
                    auth_plugin: AuthPlugin::Clear,
                    data: b"abcdefghijabcdefghij",
                },
            ),
            (
                b"\xfemysql_clear_password\x00",
                AuthSwitchRequest {
                    auth_plugin: AuthPlugin::Clear,
                    data: b"",
                },
            ),
            (
                b"\xfe\x6d\x79\x73\x71\x6c\x5f\x6e\x61\x74\x69\x76\x65\x5f\x70\x61\
                                 \x73\x73\x77\x6f\x72\x64\x00\x7a\x51\x67\x34\x69\x36\x6f\x4e\x79\
                                 \x36\x3d\x72\x48\x4e\x2f\x3e\x2d\x62\x29\x41\x00",
                AuthSwitchRequest {
                    auth_plugin: AuthPlugin::Native,
                    data: b"zQg4i6oNy6=rHN/>-b)A",
                },
            ),
        ] {
            let got = AuthSwitchRequest::deserialize(input).unwrap();
            assert_eq!(got, want);
        }
    }
}
