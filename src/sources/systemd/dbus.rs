use std::sync::atomic::{AtomicU32, Ordering};

use bytes::BufMut;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

const SYSTEMD_SOCKET_PATH: &str = "/var/run/dbus/system_bus_socket";

pub struct Client {
    stream: UnixStream,
}

impl Client {
    pub async fn connect() -> std::io::Result<Self> {
        let mut stream = match std::env::var("DBUS_SYSTEM_BUS_ADDRESS") {
            Ok(val) => UnixStream::connect(val).await?,
            _ => UnixStream::connect(SYSTEMD_SOCKET_PATH).await?,
        };

        let mut buf = [0u8; 512];

        stream.write_all(b"\0AUTH EXTERNAL\r\n").await?;
        let size = stream.read(&mut buf).await?;
        if &buf[..size] != b"DATA\r\n" {
            return Err(std::io::Error::other(format!(
                "external auth failed, resp: {:?}",
                &buf[..size]
            )));
        }

        stream.write_all(b"DATA\r\n").await?;
        let size = stream.read(&mut buf).await?;
        if !buf[..size].starts_with(b"OK ") {
            return Err(std::io::Error::other(format!(
                "DATA exchange failed, resp: {:?}",
                &buf[..size]
            )));
        }

        stream.write_all(b"NEGOTIATE_UNIX_FD\r\n").await?;
        let size = stream.read(&mut buf).await?;
        if &buf[..size] != b"AGREE_UNIX_FD\r\n" {
            return Err(std::io::Error::other(format!(
                "negotiate unix fd failed, resp: {:?}",
                &buf[..size]
            )));
        }

        stream.write_all(b"BEGIN\r\n").await?;

        let cmd = build_message(
            "/org/freedesktop/DBus",
            "Hello",
            "org.freedesktop.DBus",
            "org.freedesktop.DBus",
            &[],
        );
        stream.write_all(&cmd).await?;
        let _size = stream.read(&mut buf).await?;

        Ok(Client { stream })
    }

    pub async fn call<T: Variant>(
        &mut self,
        path: &str,
        method: &str,
        dest: &str,
        interface: &str,
        body: &[&str],
    ) -> Result<T, Error> {
        let req = build_message(path, method, dest, interface, body);

        self.stream.write_all(req.as_slice()).await?;

        let mut header = [0u8; 16];
        let size = self.stream.read(&mut header).await?;
        if size < 16 {
            return Err(Error::ResponseTooShort);
        }

        let body_len = u32::from_le_bytes((&header[4..8]).try_into().unwrap()) as usize;
        let header_len = u32::from_le_bytes((&header[12..16]).try_into().unwrap()) as usize;

        let mut resp = vec![0u8; header_len + padding(header_len, 8) + body_len];
        let mut size = 0;
        loop {
            if size == resp.capacity() {
                break;
            }

            let count = self.stream.read(&mut resp[size..]).await?;
            size += count;
        }

        T::decode(&resp[resp.len() - body_len..])
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(std::io::Error),

    #[error("signature not match")]
    SignatureNotMatch,

    #[error("body is too small")]
    BodyTooSmall,

    #[error("response is too short")]
    ResponseTooShort,
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

pub trait Variant: Sized {
    fn decode(input: &[u8]) -> Result<Self, Error>;
}

impl Variant for () {
    fn decode(_: &[u8]) -> Result<(), Error> {
        Ok(())
    }
}

impl Variant for String {
    fn decode(input: &[u8]) -> Result<Self, Error> {
        if input.len() < 9 {
            return Err(Error::BodyTooSmall);
        }

        if input[..4] != [1, 115, 0, 0] {
            return Err(Error::SignatureNotMatch);
        }

        // since align is 4 so we can handle [4..8]
        let len = u32::from_le_bytes((&input[4..8]).try_into().unwrap());

        Ok(String::from_utf8_lossy(&input[8..8 + len as usize]).to_string())
    }
}

impl Variant for u32 {
    fn decode(input: &[u8]) -> Result<Self, Error> {
        if input.len() != 8 {
            return Err(Error::BodyTooSmall);
        }

        if input[..4] != [1, 117, 0, 0] {
            return Err(Error::SignatureNotMatch);
        }

        Ok(u32::from_le_bytes([input[4], input[5], input[6], input[7]]))
    }
}

impl Variant for u64 {
    fn decode(input: &[u8]) -> Result<Self, Error> {
        if input.len() != 16 {
            return Err(Error::BodyTooSmall);
        }

        if input[0..8] != [1, 116, 0, 0, 0, 0, 0, 0] {
            return Err(Error::SignatureNotMatch);
        }

        Ok(u64::from_le_bytes((&input[8..16]).try_into().unwrap()))
    }
}

impl<const N: usize> Variant for [u64; N] {
    fn decode(input: &[u8]) -> Result<Self, Error> {
        let len = input[0] as usize;

        // something like `(ttt)`
        if len != N + 2 {
            return Err(Error::SignatureNotMatch);
        }

        let signature_len = 1 + 1 + N + 1;
        if signature_len + padding(signature_len, 4) + 8 * N != input.len() {
            return Err(Error::BodyTooSmall);
        }

        let mut resp = [0u64; N];
        for i in 0..N {
            let value = u64::from_le_bytes((&input[i * 8..i * 8 + 8]).try_into().unwrap());
            resp[i] = value;
        }

        Ok(resp)
    }
}

fn build_message(path: &str, method: &str, dest: &str, interface: &str, body: &[&str]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(256);

    // little endian
    buf.push(b'l');
    // method call
    buf.push(1);
    // flags
    buf.push(0);
    // version
    buf.push(1);
    // body length
    buf.put_slice(&[0, 0, 0, 0]);
    // serial
    let serial = SERIAL_GENERATOR.fetch_add(1, Ordering::Acquire);
    buf.put_slice(serial.to_le_bytes().as_ref());

    // headers data length
    // 4 bytes
    buf.put_slice(&[0, 0, 0, 0]);

    // path
    let pad = padding(buf.len(), 8);
    if pad > 0 {
        buf.put_bytes(0, pad)
    }
    buf.push(1);
    buf.put_slice(&[1u8, 111, 0]); // object path signature
    buf.put_slice((path.len() as u32).to_le_bytes().as_ref());
    buf.put_slice(path.as_bytes());
    buf.push(0);

    // destination
    let pad = padding(buf.len(), 8);
    if pad > 0 {
        buf.put_bytes(0, pad)
    }
    buf.push(6);
    buf.put_slice(&[1u8, 115, 0]);
    buf.put_slice((dest.len() as u32).to_le_bytes().as_ref());
    buf.put_slice(dest.as_bytes());
    buf.push(0);

    // member
    let pad = padding(buf.len(), 8);
    if pad > 0 {
        buf.put_bytes(0, pad)
    }
    buf.push(3);
    buf.put_slice(&[1u8, 115, 0]);
    buf.put_slice((method.len() as u32).to_le_bytes().as_ref());
    buf.put_slice(method.as_bytes());
    buf.push(0);

    // interface
    if !interface.is_empty() {
        let pad = padding(buf.len(), 8);
        if pad > 0 {
            buf.put_bytes(0, pad)
        }

        buf.push(2);
        buf.put_slice(&[1u8, 115, 0]);
        buf.put_slice((interface.len() as u32).to_le_bytes().as_ref());
        buf.put_slice(interface.as_bytes());
        buf.push(0);
    }

    /*
            // sender
            if !self.name.is_empty() {
                let pad = padding(buf.len(), 8);
                if pad > 0 {
                    buf.put_bytes(0, pad)
                }

                buf.push(7);
                buf.put_slice(&[1u8, 115, 0]); // string signature
                buf.put_slice((self.name.len() as u32).to_le_bytes().as_ref());
                buf.put_slice(self.name.as_bytes());
                buf.push(0);
            }
    */

    // signature
    if !body.is_empty() {
        let pad = padding(buf.len(), 8);
        if pad > 0 {
            buf.put_bytes(0, pad)
        }

        buf.push(8);
        buf.put_slice(&[1, 103, 0]);

        // NOTE: this is fine for 2 str in body
        let len = body.len();
        buf.push(len as u8);
        buf.put_bytes(b's', len);
        buf.push(0);
    }

    let headers_len = (buf.len() - 16) as u32;
    buf[12..16].copy_from_slice(&headers_len.to_le_bytes());

    // body
    let pad = padding(buf.len(), 8);
    if pad > 0 {
        buf.put_bytes(0, pad)
    }
    if !body.is_empty() {
        let body = encode_strings(body);

        buf.put_slice(body.as_slice());

        let body_len = body.len() as u32;
        buf[4..8].copy_from_slice(&body_len.to_le_bytes());
    }

    buf
}

#[inline]
pub fn padding(offset: usize, align: usize) -> usize {
    if offset % align != 0 {
        ((offset + align - 1) & (!(align - 1))) - offset
    } else {
        0
    }
}

static SERIAL_GENERATOR: AtomicU32 = AtomicU32::new(1);

#[inline]
fn encode_strings(input: &[&str]) -> Vec<u8> {
    let mut buf = Vec::new();

    for s in input {
        let pad = padding(buf.len(), 4);
        if pad > 0 {
            buf.put_bytes(0, pad);
        }

        buf.put_slice((s.len() as u32).to_le_bytes().as_ref());
        buf.put_slice(s.as_bytes());
        buf.push(0);
    }

    buf
}
