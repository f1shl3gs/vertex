use std::io;
use std::net::SocketAddr;

use bytes::{BufMut, BytesMut};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use super::{AuthConfig, Connection};

type MockRows = Vec<Vec<&'static str>>;

const SERVER_CAPABILITIES: u32 = 0x0008_8200;
const SERVER_STATUS_AUTOCOMMIT: u16 = 0x0002;
const COLUMN_TYPE_VAR_STRING: u8 = 0xfd;
const COLUMN_FLAG_NOT_NULL: u16 = 0x0001;

pub(super) struct MockServer<F> {
    listener: TcpListener,
    handler: F,
}

impl<F> MockServer<F>
where
    F: FnMut(&str) -> (Vec<&'static str>, MockRows),
{
    pub(super) async fn new(socket: SocketAddr, handler: F) -> io::Result<Self> {
        let listener = TcpListener::bind(socket).await?;

        Ok(Self { listener, handler })
    }

    async fn run(mut self) -> io::Result<()> {
        let (mut stream, _) = self.listener.accept().await?;

        send_handshake(&mut stream).await?;
        let _ = recv_packet(&mut stream).await?;
        send_ok(&mut stream, 2, 0, 0).await?;

        loop {
            let (sequence, packet) = recv_packet(&mut stream).await?;
            if packet.is_empty() {
                continue;
            }

            match packet[0] {
                0x01 => return Ok(()),
                0x03 => {
                    let query = std::str::from_utf8(&packet[1..]).map_err(invalid_data)?;
                    let (columns, rows) = (self.handler)(query);
                    assert!(!columns.is_empty());

                    for row in &rows {
                        assert_eq!(row.len(), columns.len());
                    }

                    send_result_set(&mut stream, sequence.wrapping_add(1), &columns, rows).await?;
                }
                cmd => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("unsupported command byte: 0x{cmd:02x}"),
                    ));
                }
            }
        }
    }
}

fn invalid_data(err: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, err.to_string())
}

async fn recv_packet(stream: &mut TcpStream) -> io::Result<(u8, Vec<u8>)> {
    let mut header = [0u8; 4];
    stream.read_exact(&mut header).await?;

    let len = header[0] as usize | ((header[1] as usize) << 8) | ((header[2] as usize) << 16);
    let mut payload = vec![0; len];
    stream.read_exact(&mut payload).await?;

    Ok((header[3], payload))
}

async fn send_packet(stream: &mut TcpStream, sequence: u8, payload: &[u8]) -> io::Result<()> {
    let len = payload.len();
    if len > 0x00ff_ffff {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("payload too large: {len}"),
        ));
    }

    let header = [
        (len & 0xff) as u8,
        ((len >> 8) & 0xff) as u8,
        ((len >> 16) & 0xff) as u8,
        sequence,
    ];

    stream.write_all(&header).await?;
    stream.write_all(payload).await?;
    Ok(())
}

async fn send_handshake(stream: &mut TcpStream) -> io::Result<()> {
    let mut payload = BytesMut::new();
    payload.put_u8(10);
    payload.put_slice(b"8.0.36-mock\0");
    payload.put_u32_le(1);
    payload.put_slice(b"12345678");
    payload.put_u8(0);
    payload.put_u16_le((SERVER_CAPABILITIES & 0xffff) as u16);
    payload.put_u8(45);
    payload.put_u16_le(SERVER_STATUS_AUTOCOMMIT);
    payload.put_u16_le((SERVER_CAPABILITIES >> 16) as u16);
    payload.put_u8(21);
    payload.put_bytes(0, 10);
    payload.put_slice(b"abcdefghijkl");
    payload.put_u8(0);
    payload.put_slice(b"mysql_native_password\0");

    send_packet(stream, 0, &payload).await
}

async fn send_ok(
    stream: &mut TcpStream,
    sequence: u8,
    affected_rows: u64,
    last_insert_id: u64,
) -> io::Result<()> {
    let mut payload = BytesMut::new();
    payload.put_u8(0x00);
    put_lenenc_int(&mut payload, affected_rows)?;
    put_lenenc_int(&mut payload, last_insert_id)?;
    payload.put_u16_le(SERVER_STATUS_AUTOCOMMIT);
    payload.put_u16_le(0);

    send_packet(stream, sequence, &payload).await
}

async fn send_eof(stream: &mut TcpStream, sequence: u8) -> io::Result<()> {
    let mut payload = BytesMut::new();
    payload.put_u8(0xfe);
    payload.put_u16_le(SERVER_STATUS_AUTOCOMMIT);
    payload.put_u16_le(0);

    send_packet(stream, sequence, &payload).await
}

async fn send_result_set(
    stream: &mut TcpStream,
    mut sequence: u8,
    columns: &[&'static str],
    rows: MockRows,
) -> io::Result<()> {
    let mut header = BytesMut::new();
    put_lenenc_int(&mut header, columns.len() as u64)?;
    send_packet(stream, sequence, &header).await?;
    sequence = sequence.wrapping_add(1);

    let mut buf = BytesMut::new();
    for column in columns {
        buf.clear();

        put_lenenc_str(&mut buf, b"def")?;
        put_lenenc_str(&mut buf, b"mock")?;
        put_lenenc_str(&mut buf, b"mock")?;

        put_lenenc_str(&mut buf, column.as_bytes())?;
        put_lenenc_str(&mut buf, column.as_bytes())?;
        put_lenenc_str(&mut buf, column.as_bytes())?;
        buf.put_u8(0x0c);
        buf.put_u16_le(45);
        buf.put_u32_le(1024);
        buf.put_u8(COLUMN_TYPE_VAR_STRING);
        buf.put_u16_le(COLUMN_FLAG_NOT_NULL);
        buf.put_u8(0);
        buf.put_u16_le(0);

        send_packet(stream, sequence, &buf).await?;
        sequence = sequence.wrapping_add(1);
    }

    for row in rows {
        if row.len() != columns.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "row width {} does not match column count {}",
                    row.len(),
                    columns.len()
                ),
            ));
        }

        let mut payload = BytesMut::new();
        for value in row {
            let value = value.as_bytes();
            put_lenenc_str(&mut payload, value)?;
        }

        send_packet(stream, sequence, &payload).await?;
        sequence = sequence.wrapping_add(1);
    }

    send_eof(stream, sequence).await
}

fn put_lenenc_int(buf: &mut BytesMut, value: u64) -> io::Result<()> {
    match value {
        0..=250 => buf.put_u8(value as u8),
        251..=0xffff => {
            buf.put_u8(0xfc);
            buf.put_u16_le(value as u16);
        }
        0x1_0000..=0xff_ffff => {
            buf.put_u8(0xfd);
            buf.put_uint_le(value, 3);
        }
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("unsupported lenenc integer: {value}"),
            ));
        }
    }

    Ok(())
}

fn put_lenenc_str(buf: &mut BytesMut, value: &[u8]) -> io::Result<()> {
    put_lenenc_int(buf, value.len() as u64)?;
    buf.put_slice(value);
    Ok(())
}

pub async fn mock<H>(h: H) -> Connection
where
    H: FnMut(&str) -> (Vec<&'static str>, MockRows) + Send + 'static,
{
    let server = MockServer::new(SocketAddr::from(([127, 0, 0, 1], 0)), h)
        .await
        .unwrap();
    let addr = server.listener.local_addr().unwrap();

    tokio::spawn(async move {
        let _ = server.run().await;
    });

    let auth = AuthConfig {
        username: "root".to_string(),
        password: Some("password".to_string()),
    };

    Connection::connect(addr, &auth).await.unwrap()
}

#[cfg(test)]
mod tests {
    use super::super::{AuthConfig, Connection};
    use super::*;

    #[tokio::test]
    async fn handler_returns_rows_for_query() {
        let server = MockServer::new(SocketAddr::from(([127, 0, 0, 1], 0)), |query| {
            if query == "show global status" {
                (
                    vec!["name", "value"],
                    vec![vec!["Threads_running", "3"], vec!["Uptime", "42"]],
                )
            } else {
                (vec![], Vec::new())
            }
        })
        .await
        .unwrap();

        let addr = server.listener.local_addr().unwrap();
        let task = tokio::spawn(server.run());

        let auth = AuthConfig {
            username: "root".to_string(),
            password: Some("password".to_string()),
        };
        let mut conn = Connection::connect(addr, &auth).await.unwrap();
        let mut rows = conn.query("show global status").await.unwrap();

        let mut values = Vec::new();
        while let Some(mut row) = rows.next().await.unwrap() {
            values.push((row.get_str().to_owned(), row.get_str().to_owned()));
        }

        assert_eq!(
            values,
            vec![
                ("Threads_running".to_string(), "3".to_string()),
                ("Uptime".to_string(), "42".to_string()),
            ]
        );

        conn.close().await.unwrap();
        task.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn handler_can_return_u32_values() {
        let server = MockServer::new(SocketAddr::from(([127, 0, 0, 1], 0)), |query| {
            if query == "select connections" {
                (vec!["connections"], vec![vec!["42"]])
            } else {
                (vec!["connections"], Vec::new())
            }
        })
        .await
        .unwrap();
        let addr = server.listener.local_addr().unwrap();
        let task = tokio::spawn(server.run());

        let auth = AuthConfig {
            username: "root".to_string(),
            password: None,
        };
        let mut conn = Connection::connect(addr, &auth).await.unwrap();
        let mut rows = conn.query("select connections").await.unwrap();

        let mut row = rows.next().await.unwrap().unwrap();
        assert_eq!(row.get_str(), "42");
        assert!(rows.next().await.unwrap().is_none());

        conn.close().await.unwrap();
        task.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn handler_can_return_empty_rows() {
        let server = MockServer::new(SocketAddr::from(([127, 0, 0, 1], 0)), |_| {
            (vec!["value"], Vec::new())
        })
        .await
        .unwrap();
        let addr = server.listener.local_addr().unwrap();
        let task = tokio::spawn(server.run());

        let auth = AuthConfig {
            username: "root".to_string(),
            password: None,
        };
        let mut conn = Connection::connect(addr, &auth).await.unwrap();
        let mut rows = conn.query("select 1").await.unwrap();

        assert!(rows.next().await.unwrap().is_none());

        conn.close().await.unwrap();
        task.await.unwrap().unwrap();
    }
}
