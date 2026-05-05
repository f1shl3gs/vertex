use bytes::{Buf, BytesMut};
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;

use super::packet::{EofPacket, OkPacket, get_lenenc};
use super::{Deserialize, Error};

/// There are lots of other fields, which we don't need
///
/// https://dev.mysql.com/doc/dev/mysql-server/8.0.12/page_protocol_com_query_response_text_resultset_column_definition.html
/// https://mariadb.com/docs/server/reference/clientserver-protocol/4-server-response-packets/result-set-packets#column-count-packet
/// https://dev.mysql.com/doc/internals/en/com-query-response.html#packet-Protocol::ColumnDefinition41
#[derive(Debug)]
#[allow(dead_code)]
pub struct ColumnDefinition {
    /// Since, this column definition will be used across all rows
    ///
    /// Memory layout:
    ///   | -- schema -- | -- table -- | -- name -- |
    data: Vec<u8>,

    /// we don't need usize, u8::MAX is big enough cause the
    /// schema/table/field name's length is limited at 64
    table_start: u16,
    name_start: u16,
}

#[allow(dead_code)]
impl ColumnDefinition {
    pub fn schema(&self) -> &str {
        // SAFETY: utf8 is ensured by HandshakeResponse.charset
        unsafe { std::str::from_utf8_unchecked(&self.data[..self.table_start as usize]) }
    }

    pub fn table(&self) -> &str {
        // SAFETY: utf8 is ensured by HandshakeResponse.charset
        unsafe {
            std::str::from_utf8_unchecked(
                &self.data[self.table_start as usize..self.name_start as usize],
            )
        }
    }

    pub fn name(&self) -> &str {
        // SAFETY: utf8 is ensured by HandshakeResponse.charset
        unsafe { std::str::from_utf8_unchecked(&self.data[self.name_start as usize..]) }
    }
}

impl<'de> Deserialize<'de> for ColumnDefinition {
    fn deserialize(buf: &'de [u8]) -> Result<Self, Error> {
        let mut pos = 0;
        let mut data = Vec::with_capacity(buf.len() / 2);

        // skip catalog
        pos += buf[pos] as usize + 1;

        // schema
        let len = buf[pos] as usize;
        data.extend_from_slice(&buf[pos + 1..pos + 1 + len]);
        pos += 1 + len;
        let table_start = data.len() as u16;

        // skip table alias
        pos += buf[pos] as usize + 1;

        // string<lenenc> table
        let len = buf[pos] as usize;
        data.extend_from_slice(&buf[pos + 1..pos + 1 + len]);
        pos += 1 + len;

        // skip column alias
        pos += buf[pos] as usize + 1;

        // string<lenenc> column
        let len = buf[pos] as usize;
        let name_start = data.len() as u16;
        data.extend_from_slice(&buf[pos + 1..pos + 1 + len]);
        // pos += 1 + len;

        Ok(ColumnDefinition {
            data,
            table_start,
            name_start,
        })
    }
}

/// Row is a parser of TEXT protocol
pub struct Row<'a> {
    columns: &'a [ColumnDefinition],

    data: &'a [u8],
    pos: usize,
}

impl<'a> Row<'a> {
    pub fn columns(&self) -> &'a [ColumnDefinition] {
        self.columns
    }

    pub fn reset(&mut self) {
        self.pos = 0;
    }

    pub fn get_str(&mut self) -> &'a str {
        let len = get_lenenc(self.data, &mut self.pos).expect("valid lenenc integer") as usize;
        if self.pos + len > self.data.len() {
            return "";
        }

        let start = self.pos;
        self.pos += len;

        // SAFETY: utf8 is ensured by HandshakeResponse.charset
        unsafe { std::str::from_utf8_unchecked(&self.data[start..self.pos]) }
    }
}

pub struct Rows<'a> {
    pub columns: Vec<ColumnDefinition>,

    pub stream: &'a mut TcpStream,
    pub buf: &'a mut BytesMut,
}

impl<'a> Rows<'a> {
    // Since lending stream is not really support yet, so have to
    // mimic the Stream API
    pub async fn next(&mut self) -> Result<Option<Row<'_>>, Error> {
        let mut header = [0u8; 4];

        self.stream.read_exact(&mut header).await?;

        let mut len = header[0] as u32;
        len |= (header[1] as u32) << 8;
        len |= (header[2] as u32) << 16;
        // self.sequence = header[3].wrapping_add(1);

        self.buf.resize(len as usize, 0);
        self.stream.read_exact(self.buf).await?;

        let packet = self.buf.chunk();
        if packet[0] == 0xfe {
            let (affected, last_insert, status) = if packet.len() < 9 {
                let eof = EofPacket::deserialize(packet)?;
                (0, 0, eof.status)
            } else {
                // Ok Packet
                let ok = OkPacket::deserialize(packet)?;
                (ok.affected_rows, ok.last_insert_id, ok.status)
            };

            trace!(message = "rows iterate done", affected, last_insert, status);

            return Ok(None);
        }

        Ok(Some(Row {
            columns: &self.columns,
            data: packet,
            pos: 0,
        }))
    }
}
