//! We don't need the full implement of the MessagePack, only the
//! decode methods is needed, and it's easy to implement,
//!
//! SPEC: https://github.com/msgpack/msgpack/blob/master/spec.md

use std::collections::BTreeMap;
use std::io::Read;
use std::str::Utf8Error;

use bytes::Bytes;
use chrono::{DateTime, Utc};
use value::Value;

pub trait ReadExt: Read {
    fn read_i8(&mut self) -> std::io::Result<i8> {
        let mut buf = [0; 1];
        self.read_exact(&mut buf)?;
        Ok(buf[0] as i8)
    }

    fn read_i16(&mut self) -> std::io::Result<i16> {
        let mut buf = [0; 2];
        self.read_exact(&mut buf)?;
        Ok(i16::from_be_bytes(buf))
    }

    fn read_i32(&mut self) -> std::io::Result<i32> {
        let mut buf = [0; 4];
        self.read_exact(&mut buf)?;
        Ok(i32::from_be_bytes(buf))
    }

    fn read_i64(&mut self) -> std::io::Result<i64> {
        let mut buf = [0; 8];
        self.read_exact(&mut buf)?;
        Ok(i64::from_be_bytes(buf))
    }

    fn read_u8(&mut self) -> std::io::Result<u8> {
        let mut buf = [0; 1];
        self.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    fn read_u16(&mut self) -> std::io::Result<u16> {
        let mut buf = [0; 2];
        self.read_exact(&mut buf)?;
        Ok(u16::from_be_bytes(buf))
    }

    fn read_u32(&mut self) -> std::io::Result<u32> {
        let mut buf = [0; 4];
        self.read_exact(&mut buf)?;
        Ok(u32::from_be_bytes(buf))
    }

    fn read_u64(&mut self) -> std::io::Result<u64> {
        let mut buf = [0; 8];
        self.read_exact(&mut buf)?;
        Ok(u64::from_be_bytes(buf))
    }

    fn read_f32(&mut self) -> std::io::Result<f32> {
        let mut buf = [0; 4];
        self.read_exact(&mut buf)?;
        Ok(f32::from_be_bytes(buf))
    }

    fn read_f64(&mut self) -> std::io::Result<f64> {
        let mut buf = [0; 8];
        self.read_exact(&mut buf)?;
        Ok(f64::from_be_bytes(buf))
    }
}

impl<T: Read> ReadExt for T {}

#[derive(Debug)]
pub enum Error {
    IO(std::io::Error),
    UnknownType(u8, &'static str),
    InvalidStringType(u8),
    Timestamp,
    Utf8(Utf8Error),
    EntryLength,
    UnknownExtType(u8),
    EventTimeExt(&'static str),
    UnknownOptionField(String),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IO(err)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::IO(err) => err.fmt(f),
            Error::UnknownType(v, when) => {
                write!(f, "Unknown type mark 0x{:x} found when parse {}", v, when)
            }
            Error::InvalidStringType(v) => write!(f, "Invalid string type: 0x{:x}", v),
            Error::Timestamp => write!(f, "Invalid timestamp"),
            Error::Utf8(err) => err.fmt(f),
            Error::EntryLength => f.write_str("entry length must 2"),
            Error::UnknownExtType(typ) => write!(f, "Unknown Ext type 0x{:x}", typ),
            Error::EventTimeExt(msg) => write!(f, "EventTimeExt {}", msg),
            Error::UnknownOptionField(field) => write!(f, "Unknown Option field {:?}", field),
        }
    }
}

/// https://github.com/msgpack/msgpack/blob/master/spec.md#formats
///
/// format name     | first byte (in binary) | first byte (in hex)
/// --------------- | ---------------------- | -------------------
/// positive fixint | 0xxxxxxx               | 0x00 - 0x7f
/// fixmap          | 1000xxxx               | 0x80 - 0x8f
/// fixarray        | 1001xxxx               | 0x90 - 0x9f
/// fixstr          | 101xxxxx               | 0xa0 - 0xbf
/// nil             | 11000000               | 0xc0
/// (never used)    | 11000001               | 0xc1
/// false           | 11000010               | 0xc2
/// true            | 11000011               | 0xc3
/// bin 8           | 11000100               | 0xc4
/// bin 16          | 11000101               | 0xc5
/// bin 32          | 11000110               | 0xc6
/// ext 8           | 11000111               | 0xc7
/// ext 16          | 11001000               | 0xc8
/// ext 32          | 11001001               | 0xc9
/// float 32        | 11001010               | 0xca
/// float 64        | 11001011               | 0xcb
/// uint 8          | 11001100               | 0xcc
/// uint 16         | 11001101               | 0xcd
/// uint 32         | 11001110               | 0xce
/// uint 64         | 11001111               | 0xcf
/// int 8           | 11010000               | 0xd0
/// int 16          | 11010001               | 0xd1
/// int 32          | 11010010               | 0xd2
/// int 64          | 11010011               | 0xd3
/// fixext 1        | 11010100               | 0xd4
/// fixext 2        | 11010101               | 0xd5
/// fixext 4        | 11010110               | 0xd6
/// fixext 8        | 11010111               | 0xd7
/// fixext 16       | 11011000               | 0xd8
/// str 8           | 11011001               | 0xd9
/// str 16          | 11011010               | 0xda
/// str 32          | 11011011               | 0xdb
/// array 16        | 11011100               | 0xdc
/// array 32        | 11011101               | 0xdd
/// map 16          | 11011110               | 0xde
/// map 32          | 11011111               | 0xdf
/// negative fixint | 111xxxxx               | 0xe0 - 0xff
pub fn decode_value<R: Read>(reader: &mut R) -> Result<Value, Error> {
    let mark = reader.read_u8()?;
    let value = match mark {
        // positive fixint
        0x00..=0x7f => Value::Integer(mark as i64),
        // fixmap
        0x80..=0x8f => {
            let len = mark & 0x0f;
            decode_map_with_length(reader, len as usize)?
        }
        // fixarray
        0x90..=0x9f => {
            let len = mark & 0x0f;
            decode_array_with_length(reader, len as usize)?.into()
        }
        // fixstr
        0xa0..=0xbf => {
            let len = mark & 0x1f;
            let mut buf = vec![0u8; len as usize];
            reader.read_exact(&mut buf)?;

            Value::Bytes(Bytes::from(buf))
        }

        // nil
        0xc0 => Value::Null,

        // 0xc1 is never used

        // false
        0xc2 => Value::Boolean(false),
        // true
        0xc3 => Value::Boolean(true),

        // bin 8
        0xc4 => {
            let len = reader.read_u8()? as usize;
            let mut buf = vec![0u8; len];
            reader.read_exact(&mut buf)?;

            Value::Bytes(buf.into())
        }
        // bin 16
        0xc5 => {
            let len = reader.read_u16()? as usize;
            let mut buf = vec![0u8; len];
            reader.read_exact(&mut buf)?;

            Value::Bytes(buf.into())
        }
        // bin 32
        0xc6 => {
            let len = reader.read_u32()? as usize;
            let mut buf = vec![0u8; len];
            reader.read_exact(&mut buf)?;

            Value::Bytes(buf.into())
        }

        // ext 8
        0xc7 => {
            let len = reader.read_u8()?;
            let typ = reader.read_i8()?;

            // https://github.com/msgpack/msgpack/blob/master/spec.md#timestamp-extension-type
            if typ == -1 {
                let nanos = reader.read_u32()?;
                let secs = reader.read_i64()?;

                let timestamp =
                    DateTime::<Utc>::from_timestamp(secs, nanos).ok_or(Error::Timestamp)?;
                Value::Timestamp(timestamp)
            } else if typ == 0 {
                // https://github.com/fluent/fluentd/wiki/Forward-Protocol-Specification-v1.5#eventtime-ext-format
                let secs = reader.read_i32()?;
                let nanos = reader.read_i32()?;

                let timestamp = DateTime::<Utc>::from_timestamp(secs as i64, nanos as u32)
                    .ok_or(Error::Timestamp)?;
                Value::Timestamp(timestamp)
            } else {
                let mut data = vec![0u8; len as usize];
                reader.read_exact(&mut data)?;

                let mut map = BTreeMap::new();
                map.insert("type".to_string(), typ.into());
                map.insert("data".to_string(), Bytes::from(data).into());

                Value::Object(map)
            }
        }
        // ext 16
        0xc8 => {
            let len = reader.read_u16()?;
            let typ = reader.read_i8()?;
            let mut data = vec![0u8; len as usize];
            reader.read_exact(&mut data)?;

            let mut map = BTreeMap::new();
            map.insert("type".to_string(), typ.into());
            map.insert("data".to_string(), Bytes::from(data).into());

            Value::Object(map)
        }
        // ext 32
        0xc9 => {
            let len = reader.read_u32()?;
            let typ = reader.read_i8()?;
            let mut data = vec![0u8; len as usize];
            reader.read_exact(&mut data)?;

            let mut map = BTreeMap::new();
            map.insert("type".to_string(), typ.into());
            map.insert("data".to_string(), Bytes::from(data).into());

            Value::Object(map)
        }

        // float 32
        0xca => Value::Float(reader.read_f32()? as f64),
        // float 64
        0xcb => Value::Float(reader.read_f64()?),

        // uint8
        0xcc => {
            let value = reader.read_u8()?;
            Value::Integer(value as i64)
        }
        // uint 16
        0xcd => {
            let value = reader.read_u16()?;
            Value::Integer(value as i64)
        }
        // uint 32
        0xce => {
            let value = reader.read_u32()?;
            Value::Integer(value as i64)
        }
        // uint 64
        0xcf => {
            let value = reader.read_u64()?;
            Value::Integer(value as i64)
        }

        // int 8
        0xd0 => {
            let value = reader.read_i8()?;
            Value::Integer(value as i64)
        }
        // int 16
        0xd1 => {
            let value = reader.read_i16()?;
            Value::Integer(value as i64)
        }
        // int 32
        0xd2 => {
            let value = reader.read_i32()?;
            Value::Integer(value as i64)
        }
        // int 64
        0xd3 => {
            let value = reader.read_i64()?;
            Value::Integer(value)
        }

        // fixext 1
        0xd4 => {
            let typ = reader.read_i8()?;
            let mut data = vec![0u8; 1];
            reader.read_exact(&mut data)?;

            let mut map = BTreeMap::new();
            map.insert("type".to_string(), typ.into());
            map.insert("data".to_string(), Bytes::from(data).into());

            Value::Object(map)
        }
        // fixext 2
        0xd5 => {
            let typ = reader.read_i8()?;
            let mut data = vec![0u8; 2];
            reader.read_exact(&mut data)?;

            let mut map = BTreeMap::new();
            map.insert("type".to_string(), typ.into());
            map.insert("data".to_string(), Bytes::from(data).into());

            Value::Object(map)
        }
        // fixext 4
        0xd6 => {
            let typ = reader.read_i8()?;

            // https://github.com/msgpack/msgpack/blob/master/spec.md#timestamp-extension-type
            if typ == -1 {
                let data = reader.read_u32()?;
                let timestamp =
                    DateTime::<Utc>::from_timestamp(data as i64, 0).ok_or(Error::Timestamp)?;
                Value::Timestamp(timestamp)
            } else {
                let mut data = vec![0u8; 4];
                reader.read_exact(&mut data)?;

                let mut map = BTreeMap::new();
                map.insert("type".to_string(), typ.into());
                map.insert("data".to_string(), Bytes::from(data).into());

                Value::Object(map)
            }
        }
        // fixext 8
        0xd7 => {
            let typ = reader.read_i8()?;

            if typ == -1 {
                // https://github.com/msgpack/msgpack/blob/master/spec.md#timestamp-extension-type
                let payload = reader.read_u64()?;
                let nanos = payload >> 34;
                let secs = payload & 0x3ffffffff;

                let timestamp = DateTime::<Utc>::from_timestamp(secs as i64, nanos as u32)
                    .ok_or(Error::Timestamp)?;
                Value::Timestamp(timestamp)
            } else if typ == 0 {
                // https://github.com/fluent/fluentd/wiki/Forward-Protocol-Specification-v1.5#eventtime-ext-format
                let secs = reader.read_i32()?;
                let nanos = reader.read_i32()?;

                let timestamp = DateTime::<Utc>::from_timestamp(secs as i64, nanos as u32)
                    .ok_or(Error::Timestamp)?;
                Value::Timestamp(timestamp)
            } else {
                let mut data = vec![0u8; 8];
                reader.read_exact(&mut data)?;

                let mut map = BTreeMap::new();
                map.insert("type".to_string(), typ.into());
                map.insert("data".to_string(), Bytes::from(data).into());

                Value::Object(map)
            }
        }
        // fixext 16
        0xd8 => {
            let typ = reader.read_i8()?;
            let mut data = vec![0u8; 16];
            reader.read_exact(&mut data)?;

            let mut map = BTreeMap::new();
            map.insert("type".to_string(), typ.into());
            map.insert("data".to_string(), Bytes::from(data).into());

            Value::Object(map)
        }

        // str 8
        0xd9 => {
            let len = reader.read_u8()?;
            let mut buf = vec![0u8; len as usize];
            reader.read_exact(&mut buf)?;

            Value::Bytes(buf.into())
        }
        // str 16
        0xda => {
            let len = reader.read_u16()?;
            let mut buf = vec![0u8; len as usize];
            reader.read_exact(&mut buf)?;

            Value::Bytes(buf.into())
        }
        // str 32
        0xdb => {
            let len = reader.read_u32()?;
            let mut buf = vec![0u8; len as usize];
            reader.read_exact(&mut buf)?;

            Value::Bytes(buf.into())
        }

        // array 16
        0xdc => {
            let len = reader.read_u16()?;
            decode_array_with_length(reader, len as usize)?.into()
        }
        // array 32
        0xdd => {
            let len = reader.read_u32()?;
            decode_array_with_length(reader, len as usize)?.into()
        }

        // map 16
        0xde => {
            let len = reader.read_u16()?;
            decode_map_with_length(reader, len as usize)?
        }
        // map 32
        0xdf => {
            let len = reader.read_u32()?;
            decode_map_with_length(reader, len as usize)?
        }

        // negative fixint
        0xe0..=0xff => Value::Integer(mark as i8 as i64),
        _ => return Err(Error::UnknownType(mark, "value")),
    };

    Ok(value)
}

fn decode_map_with_length<R: Read>(reader: &mut R, len: usize) -> Result<Value, Error> {
    let mut map = BTreeMap::new();

    for _ in 0..len {
        let key = decode_string(reader)?;
        let value = decode_value(reader)?;

        map.insert(key, value);
    }

    Ok(Value::Object(map))
}

pub fn decode_array_with_length<R: Read>(reader: &mut R, len: usize) -> Result<Vec<Value>, Error> {
    let mut array = Vec::with_capacity(len);

    for _ in 0..len {
        let value = decode_value(reader)?;

        array.push(value);
    }

    Ok(array)
}

pub fn decode_string<R: Read>(reader: &mut R) -> Result<String, Error> {
    let typ = reader.read_u8()?;

    let len = match typ {
        // fixstr
        0xa0..=0xbf => (typ & 0x1f) as usize,
        // str 8
        0xd9 => reader.read_u8()? as usize,
        // str 16
        0xda => reader.read_u16()? as usize,
        // str 32
        0xdb => reader.read_u32()? as usize,
        _ => return Err(Error::InvalidStringType(typ)),
    };

    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf)?;

    String::from_utf8(buf).map_err(|err| Error::Utf8(err.utf8_error()))
}

// https://github.com/fluent/fluentd/wiki/Forward-Protocol-Specification-v0#entry
// https://github.com/fluent/fluentd/wiki/Forward-Protocol-Specification-v1#entry
pub fn decode_entry<R: Read>(reader: &mut R) -> Result<(DateTime<Utc>, Value), Error> {
    let len = match reader.read_u8()? {
        // fixarray
        typ @ 0x90..=0x9f => (typ & 0x0f) as usize,
        // array 16
        0xdc => reader.read_u16()? as usize,
        // array 32
        0xdd => reader.read_u32()? as usize,
        typ => return Err(Error::UnknownType(typ, "entry")),
    };

    if len != 2 {
        return Err(Error::EntryLength);
    }

    let ts = match reader.read_u8()? {
        // fixext 8
        //
        // https://github.com/fluent/fluentd/wiki/Forward-Protocol-Specification-v1#eventtime-ext-format
        0xd7 => {
            let typ = reader.read_u8()?;
            if typ != 0 {
                return Err(Error::UnknownExtType(typ));
            }

            let secs = reader.read_i32()?;
            let nanos = reader.read_i32()?;

            DateTime::from_timestamp(secs as i64, nanos as u32).ok_or(Error::Timestamp)?
        }
        // ext 8
        //
        // https://github.com/fluent/fluentd/wiki/Forward-Protocol-Specification-v0#eventtime-ext-format
        // https://github.com/fluent/fluentd/wiki/Forward-Protocol-Specification-v1#eventtime-ext-format
        0xc7 => {
            let len = reader.read_u8()?;
            if len != 8 {
                return Err(Error::EventTimeExt("length should be 8"));
            }

            let typ = reader.read_u8()?;
            if typ != 0 {
                return Err(Error::UnknownExtType(typ));
            }

            let secs = reader.read_i32()?;
            let nanos = reader.read_i32()?;

            DateTime::from_timestamp(secs as i64, nanos as u32).ok_or(Error::Timestamp)?
        }
        // uint 32
        0xce => {
            let secs = reader.read_u32()?;

            DateTime::from_timestamp(secs as i64, 0).ok_or(Error::Timestamp)?
        }

        typ => return Err(Error::UnknownType(typ, "timestamp in entry")),
    };

    // decode map
    let value = match reader.read_u8()? {
        typ @ 0x80..=0x8f => {
            let len = typ & 0x0f;
            decode_map_with_length(reader, len as usize)?
        }
        0xde => {
            let len = reader.read_u16()?;
            decode_map_with_length(reader, len as usize)?
        }
        0xdf => {
            let len = reader.read_u32()?;
            decode_map_with_length(reader, len as usize)?
        }
        typ => return Err(Error::UnknownType(typ, "map in entry")),
    };

    Ok((ts, value))
}

pub struct Options {
    /// Clients MAY send the size option to show the number of event
    /// records in an entries by an integer as a value.
    #[allow(dead_code)]
    pub size: usize,
    pub chunk: Option<Vec<u8>>,
    // for now, only gzip available
    pub compressed: bool,
}

pub fn decode_options<R: Read>(reader: &mut R) -> Result<Options, Error> {
    let len = match reader.read_u8()? {
        typ @ 0x80..0x8f => (typ & 0x0f) as usize,
        0xde => reader.read_u16()? as usize,
        0xdf => reader.read_u32()? as usize,
        typ => return Err(Error::UnknownType(typ, "options")),
    };

    let mut size = 0;
    let mut chunk = None;
    let mut compressed = false;
    for _ in 0..len {
        let key = decode_string(reader)?;

        match key.as_str() {
            "size" => {
                size = match reader.read_u8()? {
                    typ @ 0x00..=0x7f => typ as usize,
                    0xcc => reader.read_u8()? as usize,
                    0xcd => reader.read_u16()? as usize,
                    0xce => reader.read_u32()? as usize,
                    0xcf => reader.read_u64()? as usize,
                    typ => return Err(Error::UnknownType(typ, "size in options")),
                }
            }
            "chunk" => {
                let len = match reader.read_u8()? {
                    typ @ 0xa0..=0xbf => (typ & 0x1f) as usize,
                    0xd9 => reader.read_u8()? as usize,
                    0xda => reader.read_u16()? as usize,
                    0xdb => reader.read_u32()? as usize,
                    typ => return Err(Error::UnknownType(typ, "chunk in options")),
                };

                let mut buf = vec![0u8; len];
                reader.read_exact(&mut buf)?;
                chunk = Some(buf);
            }
            "compressed" => {
                compressed = decode_string(reader)? == "gzip";
            }
            // Added in Forward Protocol Specification v1.5, which indicates the type of
            // the entry.
            //
            // https://github.com/fluent/fluentd/wiki/Forward-Protocol-Specification-v1.5
            // https://github.com/fluent/fluentd/wiki/Forward-Protocol-Specification-v1.5#types-of-entry
            "fluent_signal" => {
                let _ = match reader.read_u8()? {
                    typ @ 0x00..=0x7f => typ as usize,
                    0xcc => reader.read_u8()? as usize,
                    0xcd => reader.read_u16()? as usize,
                    0xce => reader.read_u32()? as usize,
                    0xcf => reader.read_u64()? as usize,
                    typ => return Err(Error::UnknownType(typ, "fluent_signal in options")),
                };
            }
            _ => return Err(Error::UnknownOptionField(key)),
        }
    }

    Ok(Options {
        size,
        chunk,
        compressed,
    })
}

pub fn decode_binary<R: Read>(reader: &mut R) -> Result<Vec<u8>, Error> {
    let len = match reader.read_u8()? {
        // bin 8
        0xc4 => reader.read_u8()? as usize,
        // bin 16
        0xc5 => reader.read_u16()? as usize,
        // bin 32
        0xc6 => reader.read_u32()? as usize,
        // fixint
        typ @ 0x00..=0x7f => typ as usize,
        typ => return Err(Error::UnknownType(typ, "binary")),
    };

    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf)?;

    Ok(buf)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use value::value;

    use super::*;

    #[test]
    fn simple() {
        let input = [
            0x87, 0xA3, 0x69, 0x6E, 0x74, 0x01, 0xA5, 0x66, 0x6C, 0x6F, 0x61, 0x74, 0xCB, 0x3F,
            0xE0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xA7, 0x62, 0x6F, 0x6F, 0x6C, 0x65, 0x61,
            0x6E, 0xC3, 0xA4, 0x6E, 0x75, 0x6C, 0x6C, 0xC0, 0xA6, 0x73, 0x74, 0x72, 0x69, 0x6E,
            0x67, 0xA7, 0x66, 0x6F, 0x6F, 0x20, 0x62, 0x61, 0x72, 0xA5, 0x61, 0x72, 0x72, 0x61,
            0x79, 0x92, 0xA3, 0x66, 0x6F, 0x6F, 0xA3, 0x62, 0x61, 0x72, 0xA6, 0x6F, 0x62, 0x6A,
            0x65, 0x63, 0x74, 0x82, 0xA3, 0x66, 0x6F, 0x6F, 0x01, 0xA3, 0x62, 0x61, 0x7A, 0xCB,
            0x3F, 0xE0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let want = value!({
          "int": 1,
          "float": 0.5,
          "boolean": true,
          "null": null,
          "string": "foo bar",
          "array": [
            "foo",
            "bar"
          ],
          "object": {
            "foo": 1,
            "baz": 0.5
          }
        });

        let mut reader = Cursor::new(input);
        let got = decode_value(&mut reader).unwrap();

        assert_eq!(reader.position(), input.len() as u64);
        assert_eq!(want, got);
    }

    #[test]
    fn complex() {
        let input = [
            0x84, 0xa6, 0x6e, 0x75, 0x6c, 0x6c, 0x65, 0x64, 0xc0, 0xa5, 0x62, 0x61, 0x73, 0x69,
            0x63, 0xc3, 0xa4, 0x6c, 0x69, 0x73, 0x74, 0x94, 0xc3, 0xc0, 0x93, 0xc3, 0xc0, 0xc3,
            0x82, 0xa5, 0x62, 0x61, 0x73, 0x69, 0x63, 0xc3, 0xa5, 0x62, 0x75, 0x64, 0x64, 0x79,
            0xcb, 0x3f, 0xf1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9a, 0xa3, 0x6d, 0x61, 0x70, 0x83,
            0xa5, 0x62, 0x61, 0x73, 0x69, 0x63, 0xc3, 0xa4, 0x6c, 0x69, 0x73, 0x74, 0x93, 0xc3,
            0xc0, 0xc3, 0xa3, 0x6d, 0x61, 0x70, 0x82, 0xa5, 0x62, 0x61, 0x73, 0x69, 0x63, 0xc3,
            0xa5, 0x62, 0x75, 0x64, 0x64, 0x79, 0xff,
        ];

        let want = value!({
          "nulled": null,
          "basic": true,
          "list": [
            true,
            null,
            [true, null, true],
            {
              "basic": true,
              "buddy": 1.1
            }
          ],
          "map": {
            "basic": true,
            "list": [true, null, true],
            "map": {
              "basic": true,
              "buddy": (-1)
            }
          }
        });

        let mut reader = Cursor::new(input);
        let got = decode_value(&mut reader).unwrap();

        assert_eq!(reader.position(), input.len() as u64);
        assert_eq!(want, got);
    }

    #[test]
    fn utf8() {
        let input = [0x91, 0xA7, 0xE2, 0x82, 0xAC, 0xF0, 0x9D, 0x84, 0x9E];
        let want = value!(["€𝄞"]);

        let mut reader = Cursor::new(input);
        let got = decode_value(&mut reader).unwrap();
        assert_eq!(reader.position(), input.len() as u64);
        assert_eq!(got, want);
    }

    #[test]
    fn decode() {
        for (input, want) in [
            (vec![0xc0], Value::Null),
            (vec![0x1f], Value::from(31)),
            (vec![0xc3], Value::Boolean(true)),
            (vec![0xc2], Value::Boolean(false)),
            (
                vec![0xc4, 0x08, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08],
                Value::Bytes(Bytes::from(vec![
                    0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
                ])),
            ),
            (
                vec![
                    0xc5, 0x00, 0x08, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
                ],
                Value::Bytes(vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08].into()),
            ),
            (
                vec![
                    0xc6, 0x00, 0x00, 0x00, 0x08, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
                ],
                Value::Bytes(vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08].into()),
            ),
            (vec![0xd2, 0xff, 0xff, 0xff, 0xff], Value::Integer(-1)),
            (
                vec![0xcb, 0xff, 0xf0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
                f64::NEG_INFINITY.into(),
            ),
            (
                vec![
                    0xaa, 0x6c, 0x65, 0x20, 0x6d, 0x65, 0x73, 0x73, 0x61, 0x67, 0x65,
                ],
                "le message".into(),
            ),
            (
                vec![0x93, 0x00, 0x2a, 0xf7],
                Value::Array(vec![0.into(), 42.into(), (-9).into()]),
            ),
            (
                vec![
                    0x9f, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c,
                    0x0d, 0x0e, 0x0f,
                ],
                Value::Array(vec![
                    Value::from(1),
                    Value::from(2),
                    Value::from(3),
                    Value::from(4),
                    Value::from(5),
                    Value::from(6),
                    Value::from(7),
                    Value::from(8),
                    Value::from(9),
                    Value::from(10),
                    Value::from(11),
                    Value::from(12),
                    Value::from(13),
                    Value::from(14),
                    Value::from(15),
                ]),
            ),
            (
                vec![0xdc, 0x00, 0x06, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06],
                Value::Array(vec![
                    Value::from(1),
                    Value::from(2),
                    Value::from(3),
                    Value::from(4),
                    Value::from(5),
                    Value::from(6),
                ]),
            ),
            (
                vec![
                    0xdd, 0x00, 0x00, 0x00, 0x06, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06,
                ],
                Value::Array(vec![
                    Value::from(1),
                    Value::from(2),
                    Value::from(3),
                    Value::from(4),
                    Value::from(5),
                    Value::from(6),
                ]),
            ),
            // (
            //     vec![
            //         0x82,
            //         0x2a,
            //         0xce, 0x0, 0x1, 0x88, 0x94,
            //         0xa3, 0x6b, 0x65, 0x79,
            //         0xa5, 0x76, 0x61, 0x6c, 0x75, 0x65
            //     ],
            //     value!({
            //         42: 100500,
            //         "key": "value"
            //     })
            // ),
            // (
            //     vec![
            //         0xde,
            //         0x00, 0x02,
            //         0x2a,
            //         0xce, 0x0, 0x1, 0x88, 0x94,
            //         0xa3, 0x6b, 0x65, 0x79,
            //         0xa5, 0x76, 0x61, 0x6c, 0x75, 0x65
            //     ],
            //     value!({
            //         42: 100500,
            //         "key": "value"
            //     })
            // ),
            // (
            //     vec![
            //         0xdf,
            //         0x00, 0x00, 0x00, 0x02,
            //         0x2a,
            //         0xce, 0x0, 0x1, 0x88, 0x94,
            //         0xa3, 0x6b, 0x65, 0x79,
            //         0xa5, 0x76, 0x61, 0x6c, 0x75, 0x65
            //     ],
            //     value!({
            //         42: 100500,
            //         "key": "value"
            //     })
            // ),
            (
                // fixext1
                vec![0xd4, 0x01, 0x02],
                {
                    let mut map = BTreeMap::new();
                    map.insert("type".to_string(), 1.into());
                    map.insert("data".to_string(), Value::Bytes(Bytes::from(vec![2u8])));

                    Value::Object(map)
                },
            ),
            (
                // fixext2
                vec![0xd5, 0x01, 0x02, 0x03],
                {
                    let mut map = BTreeMap::new();
                    map.insert("type".to_string(), 1.into());
                    map.insert("data".to_string(), Value::Bytes(Bytes::from(vec![2, 3])));

                    Value::Object(map)
                },
            ),
            (
                // fix ext 4
                vec![0xd6, 0x01, 0x02, 0x03, 0x04, 0x05],
                {
                    let mut map = BTreeMap::new();
                    map.insert("type".to_string(), 1.into());
                    map.insert(
                        "data".to_string(),
                        Value::Bytes(Bytes::from(vec![2, 3, 4, 5])),
                    );

                    Value::Object(map)
                },
            ),
            (
                // fix ext 8
                vec![0xd7, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09],
                {
                    let mut map = BTreeMap::new();
                    map.insert("type".to_string(), 1.into());
                    map.insert(
                        "data".to_string(),
                        Value::Bytes(Bytes::from(vec![2, 3, 4, 5, 6, 7, 8, 9])),
                    );

                    Value::Object(map)
                },
            ),
            (
                // fix ext 16
                vec![
                    0xd8, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x02, 0x03, 0x04,
                    0x05, 0x06, 0x07, 0x08, 0x09,
                ],
                {
                    let mut map = BTreeMap::new();
                    map.insert("type".to_string(), 1.into());
                    map.insert(
                        "data".to_string(),
                        Value::Bytes(Bytes::from(vec![
                            2, 3, 4, 5, 6, 7, 8, 9, 2, 3, 4, 5, 6, 7, 8, 9,
                        ])),
                    );

                    Value::Object(map)
                },
            ),
            (
                // ext 8
                vec![0xc7, 0x04, 0x01, 0x02, 0x03, 0x04, 0x05],
                {
                    let mut map = BTreeMap::new();
                    map.insert("type".to_string(), 1.into());
                    map.insert(
                        "data".to_string(),
                        Value::Bytes(Bytes::from(vec![2, 3, 4, 5])),
                    );

                    Value::Object(map)
                },
            ),
            (
                // ext 16
                vec![0xc8, 0x00, 0x04, 0x01, 0x02, 0x03, 0x04, 0x05],
                {
                    let mut map = BTreeMap::new();
                    map.insert("type".to_string(), 1.into());
                    map.insert(
                        "data".to_string(),
                        Value::Bytes(Bytes::from(vec![2, 3, 4, 5])),
                    );

                    Value::Object(map)
                },
            ),
            (
                // ext 32
                vec![0xc9, 0x00, 0x00, 0x00, 0x04, 0x01, 0x02, 0x03, 0x04, 0x05],
                {
                    let mut map = BTreeMap::new();
                    map.insert("type".to_string(), 1.into());
                    map.insert(
                        "data".to_string(),
                        Value::Bytes(Bytes::from(vec![2, 3, 4, 5])),
                    );

                    Value::Object(map)
                },
            ),
            (
                // str8
                vec![
                    0xd9, 0x20, 0x42, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x30,
                    0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x30, 0x31, 0x32, 0x33,
                    0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x30, 0x45,
                ],
                Value::Bytes("B123456789012345678901234567890E".into()),
            ),
            (
                vec![
                    0xda, 0x00, 0x20, 0x42, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39,
                    0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x30, 0x31, 0x32,
                    0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x30, 0x45,
                ],
                Value::Bytes("B123456789012345678901234567890E".into()),
            ),
            (
                vec![
                    0xdb, 0x00, 0x00, 0x00, 0x20, 0x42, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37,
                    0x38, 0x39, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x30,
                    0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x30, 0x45,
                ],
                Value::Bytes("B123456789012345678901234567890E".into()),
            ),
            (vec![0x92, 0x04, 0x2a], value!([4, 42])),
        ] {
            let len = input.len();
            let mut reader = Cursor::new(input);
            match decode_value(&mut reader) {
                Ok(got) => {
                    assert_eq!(want, got);
                    assert_eq!(reader.position() as usize, len);
                }
                Err(err) => panic!("{}", err),
            }
        }
    }
}
