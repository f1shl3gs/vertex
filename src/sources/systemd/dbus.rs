//! The D-Bus API of systemd
//!
//! https://www.freedesktop.org/wiki/Software/systemd/dbus/

use std::sync::atomic::{AtomicU32, Ordering};

use bytes::BufMut;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

const SYSTEMD_SOCKET_PATH: &str = "/var/run/dbus/system_bus_socket";

static SERIAL_GENERATOR: AtomicU32 = AtomicU32::new(1);

pub struct Client {
    stream: UnixStream,
}

impl Client {
    pub async fn connect() -> Result<Self, Error> {
        let mut stream = match std::env::var("DBUS_SYSTEM_BUS_ADDRESS") {
            Ok(val) => UnixStream::connect(val).await?,
            _ => UnixStream::connect(SYSTEMD_SOCKET_PATH).await?,
        };

        let mut buf = [0u8; 512];

        stream.write_all(b"\0AUTH EXTERNAL\r\n").await?;
        let size = stream.read(&mut buf).await?;
        if &buf[..size] != b"DATA\r\n" {
            return Err(Error::Authentication);
        }

        stream.write_all(b"DATA\r\n").await?;
        let size = stream.read(&mut buf).await?;
        if !buf[..size].starts_with(b"OK ") {
            return Err(Error::Authentication);
        }

        stream.write_all(b"BEGIN\r\n").await?;

        let mut client = Client { stream };

        let _guid = client
            .call::<Vec<u8>>(
                "/org/freedesktop/DBus",
                "Hello",
                "org.freedesktop.DBus",
                "org.freedesktop.DBus",
                &[],
            )
            .await?;

        Ok(client)
    }

    pub async fn call<T: Variant>(
        &mut self,
        path: &str,
        method: &str,
        destination: &str,
        interface: &str,
        body: &[&str],
    ) -> Result<T, Error> {
        let mut buf = build_message(path, method, destination, interface, body);

        self.stream.write_all(buf.as_slice()).await?;

        let mut header = [0u8; 16];
        loop {
            self.stream.read_exact(&mut header).await?;

            let body_len = u32::from_le_bytes((&header[4..8]).try_into().unwrap()) as usize;
            let header_len = u32::from_le_bytes((&header[12..16]).try_into().unwrap()) as usize;
            if body_len + header_len + 16 > 1 << 27 {
                return Err(Error::MessageTooBig);
            }

            let want = header_len + padding(header_len, 8) + body_len;
            buf.truncate(0);
            buf.reserve(want);

            let mut size = 0;
            loop {
                let uninitialed = unsafe {
                    std::slice::from_raw_parts_mut(buf.as_mut_ptr().add(size), want - size)
                };

                size += self.stream.read(uninitialed).await?;
                if size == want {
                    unsafe { buf.set_len(want) };
                    break;
                }
            }

            // we don't case about signals
            if header[1] == 4 {
                continue;
            }

            // maybe we should validate response

            break T::decode(&buf[want - body_len..]);
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("authentication failed")]
    Authentication,

    #[error("signature not match")]
    SignatureNotMatch,

    #[error("message is too large")]
    MessageTooBig,

    #[error("body is too small")]
    BodyTooSmall,
}

pub trait Variant: Sized {
    fn decode(input: &[u8]) -> Result<Self, Error>;
}

impl Variant for Vec<u8> {
    fn decode(input: &[u8]) -> Result<Self, Error> {
        if input.len() < 4 {
            return Err(Error::BodyTooSmall);
        }

        let len = u32::from_le_bytes([input[0], input[1], input[2], input[3]]) as usize;

        Ok(Vec::from(&input[4..4 + len]))
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

fn build_message(
    path: &str,
    method: &str,
    destination: &str,
    interface: &str,
    body: &[&str],
) -> Vec<u8> {
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

    // interface
    let pad = padding(buf.len(), 8);
    if pad > 0 {
        buf.put_bytes(0, pad)
    }
    buf.push(2);
    buf.put_slice(&[1u8, 115, 0]);
    buf.put_slice((interface.len() as u32).to_le_bytes().as_ref());
    buf.put_slice(interface.as_bytes());
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

    // destination
    let pad = padding(buf.len(), 8);
    if pad > 0 {
        buf.put_bytes(0, pad)
    }
    buf.push(6);
    buf.put_slice(&[1u8, 115, 0]);
    buf.put_slice((destination.len() as u32).to_le_bytes().as_ref());
    buf.put_slice(destination.as_bytes());
    buf.push(0);

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
        let start = buf.len();
        for arg in body {
            let pad = padding(buf.len(), 4);
            if pad > 0 {
                buf.put_bytes(0, pad);
            }

            buf.put_slice((arg.len() as u32).to_le_bytes().as_ref());
            buf.put_slice(arg.as_bytes());
            buf.push(0);
        }

        let body_len = (buf.len() - start) as u32;
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
