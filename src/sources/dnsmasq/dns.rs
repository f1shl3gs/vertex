#![allow(dead_code)]

use std::io::{Read, Seek, SeekFrom, Write};

pub trait Encodable: Sized {
    fn encode<W: Write>(&self, buf: &mut W) -> Result<(), EncodeError>;

    fn decode<R: Read + Seek>(buf: &mut R) -> Result<Self, DecodeError>;
}

#[derive(Debug, Default)]
pub struct Header {
    pub id: u16,
    pub response: bool,
    pub opcode: i32,
    pub authoritative: bool,
    pub truncated: bool,
    pub recursion_desired: bool,
    pub recursion_available: bool,
    pub zero: bool,
    pub authenticated_data: bool,
    pub checking_disabled: bool,
    pub rcode: i32,
}

#[derive(Debug)]
pub struct Question {
    pub name: String,
    pub typ: u16,
    pub class: u16,
}

impl Encodable for Question {
    fn encode<W: Write>(&self, buf: &mut W) -> Result<(), EncodeError> {
        self.name.split(".").try_for_each(|part| {
            buf.write_all(&[part.len() as u8])?;
            buf.write_all(part.as_bytes())
        })?;

        buf.write_all(&self.typ.to_be_bytes())?;
        buf.write_all(&self.class.to_be_bytes())?;

        Ok(())
    }

    fn decode<R: Read + Seek>(reader: &mut R) -> Result<Self, DecodeError> {
        let mut name = String::new();
        let mut buf = [0u8; 256];

        loop {
            reader.read_exact(&mut buf[..1])?;
            let len = buf[0] as usize;
            if len == 0 {
                break;
            }

            reader.read_exact(&mut buf[..len])?;

            name.push_str(unsafe { std::str::from_utf8_unchecked(&buf[..len]) });
            name.push('.');
        }

        let mut raw = [0u8; 2];
        reader.read_exact(&mut raw)?;
        let typ = u16::from_be_bytes(raw);

        reader.read_exact(&mut raw)?;
        let class = u16::from_be_bytes(raw);

        Ok(Question { name, typ, class })
    }
}

#[derive(Debug)]
pub struct RR {
    pub name: String,
    pub typ: u16,
    pub class: u16,
    pub ttl: u32,
    pub rd_length: u16, // Length of data after header.

    pub data: Vec<u8>,
}

impl Encodable for RR {
    fn encode<W: Write>(&self, _buf: &mut W) -> Result<(), EncodeError> {
        // for now
        unreachable!()
    }

    fn decode<R: Read + Seek>(reader: &mut R) -> Result<Self, DecodeError> {
        let mut name = String::new();
        let mut buf = [0u8; 256];
        let mut offset = None;

        reader.read_exact(&mut buf[..1])?;
        match buf[0] & 0xc0 {
            0x00 => {
                reader.seek_relative(-1)?;
            }
            0xc0 => {
                // pointer to somewhere in this msg
                reader.read_exact(&mut buf[..1])?;
                offset = Some(reader.stream_position()?);
                reader.seek(SeekFrom::Start(buf[0] as u64))?;
            }
            _ => return Err(DecodeError::BadRecordData),
        }

        loop {
            reader.read_exact(&mut buf[..1])?;
            let len = buf[0] as usize;
            if len == 0 {
                break;
            }

            reader.read_exact(&mut buf[..len])?;

            name.push_str(unsafe { std::str::from_utf8_unchecked(&buf[..len]) });
            name.push('.');
        }

        if let Some(offset) = offset {
            reader.seek(SeekFrom::Start(offset))?;
        }

        let mut two = [0u8; 2];
        reader.read_exact(&mut two)?;
        let typ = u16::from_be_bytes(two);

        reader.read_exact(&mut two)?;
        let class = u16::from_be_bytes(two);

        let mut four = [0u8; 4];
        reader.read_exact(&mut four)?;
        let ttl = u32::from_be_bytes(four);

        reader.read_exact(&mut two)?;
        let rd_length = u16::from_be_bytes(two);

        reader.read_exact(&mut buf[..rd_length as usize])?;

        Ok(RR {
            name,
            typ,
            class,
            ttl,
            rd_length,
            data: buf[..rd_length as usize].to_vec(),
        })
    }
}

#[derive(Debug, Default)]
pub struct Message {
    pub header: Header,

    pub questions: Vec<Question>,
    pub answers: Vec<RR>,
    pub ns: Vec<RR>,
    pub extra: Vec<RR>,
}

#[derive(Debug)]
pub enum EncodeError {
    InvalidRcode(i32),

    Io(std::io::Error),
}

impl From<std::io::Error> for EncodeError {
    fn from(err: std::io::Error) -> EncodeError {
        EncodeError::Io(err)
    }
}

#[derive(Debug)]
pub enum DecodeError {
    Io(std::io::Error),

    BadRecordData,
}

impl From<std::io::Error> for DecodeError {
    fn from(err: std::io::Error) -> DecodeError {
        DecodeError::Io(err)
    }
}

impl Encodable for Message {
    fn encode<W: Write>(&self, buf: &mut W) -> Result<(), EncodeError> {
        if self.header.rcode < 0 || self.header.rcode > 0xFFF {
            return Err(EncodeError::InvalidRcode(self.header.rcode));
        }

        let mut bits = ((self.header.opcode as u16) << 11) | (self.header.rcode & 0xF) as u16;
        if self.header.response {
            bits |= 1 << 15; // query/response
        }
        if self.header.authoritative {
            bits |= 1 << 10; // authoritative
        }
        if self.header.truncated {
            bits |= 1 << 9; // truncated
        }
        if self.header.recursion_desired {
            bits |= 1 << 8; // recursion desired
        }
        if self.header.recursion_available {
            bits |= 1 << 7;
        }
        if self.header.zero {
            bits |= 1 << 6;
        }
        if self.header.authenticated_data {
            bits |= 1 << 5;
        }
        if self.header.checking_disabled {
            bits |= 1 << 4;
        }

        let qd_count = self.questions.len() as u16;
        let an_count = 0u16;
        let ns_count = 0u16;
        let ar_count = 0u16;

        buf.write_all(&self.header.id.to_be_bytes())?;
        buf.write_all(&bits.to_be_bytes())?;
        buf.write_all(&qd_count.to_be_bytes())?;
        buf.write_all(&an_count.to_be_bytes())?;
        buf.write_all(&ns_count.to_be_bytes())?;
        buf.write_all(&ar_count.to_be_bytes())?;

        for question in &self.questions {
            question.encode(buf)?;
        }

        Ok(())
    }

    fn decode<R: Read + Seek>(buf: &mut R) -> Result<Message, DecodeError> {
        let mut two = [0u8; 2];

        buf.read_exact(&mut two)?;
        let id = u16::from_be_bytes(two);
        buf.read_exact(&mut two)?;
        let _bits = u16::from_be_bytes(two);
        buf.read_exact(&mut two)?;
        let qd_count = u16::from_be_bytes(two);
        buf.read_exact(&mut two)?;
        let an_count = u16::from_be_bytes(two);
        buf.read_exact(&mut two)?;
        let ns_count = u16::from_be_bytes(two);
        buf.read_exact(&mut two)?;
        let ar_count = u16::from_be_bytes(two);

        let mut questions = Vec::with_capacity(qd_count as usize);
        for _ in 0..qd_count {
            questions.push(Question::decode(buf)?);
        }

        let mut answers = Vec::with_capacity(an_count as usize);
        for _ in 0..an_count {
            answers.push(RR::decode(buf)?);
        }

        let mut authority = Vec::with_capacity(ns_count as usize);
        for _ in 0..ns_count {
            authority.push(RR::decode(buf)?);
        }

        let mut additional = Vec::with_capacity(ar_count as usize);
        for _ in 0..ar_count {
            additional.push(RR::decode(buf)?);
        }

        Ok(Message {
            header: Header {
                id,
                ..Default::default()
            },
            questions,
            answers,
            ns: authority,
            extra: additional,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn encode() {
        let msg = Message {
            header: Header {
                id: 11,
                recursion_desired: true,
                ..Default::default()
            },
            questions: vec![Question {
                name: "cachesize.bind.".to_string(),
                typ: 16,
                class: 3,
            }],
            answers: vec![],
            ns: vec![],
            extra: vec![],
        };

        let mut buf = Cursor::new(Vec::with_capacity(512));
        msg.encode(&mut buf).unwrap();
        println!("size: {:?}", buf.into_inner());

        /*
        RUST
        [0, 11, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 9, 99, 97, 99, 104, 101, 115, 105, 122, 101, 4, 98, 105, 110, 100, 0, 0, 16, 0, 3]
        GO
        [0, 11, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 9, 99, 97, 99, 104, 101, 115, 105, 122, 101, 4, 98, 105, 110, 100, 0, 0, 16, 0, 3]
        */
    }

    #[test]
    fn decode() {
        #[rustfmt::skip]
        let input: [u8; 48] = [
            // id
            0, 1,
            // bits
            133, 128,
            // question count
            0, 1,
            // answer count
            0, 1,
            // name server count
            0, 0,
            // additional count
            0, 0,
            // Questions
            // names
            // part length
            9,
            // part content
            99, 97, 99, 104, 101, 115, 105, 122, 101,
            // part length
            4,
            // part content
            98, 105, 110, 100,
            // part length, zero means it's end
            0,
            // question type TXT
            0, 16,
            // Class type CHAOS
            0, 3,
            // Answers
            // Name, pointer
            192, 12,
            // question type TXT
            0, 16,
            // Class
            0, 3,
            // TTL
            0, 0, 0, 0,
            // rd length
            0, 4,
            // rd data
            3, 49, 50, 56,
        ];

        let msg = Message::decode(&mut Cursor::new(input)).unwrap();

        println!("{:#?}", msg);
    }

    #[test]
    fn ptr() {
        let n = 192u8;
        let np = 12u8;

        let n = (((n ^ 0xc0) as i32) << 8) | np as i32;

        println!("ptr {}", n);
    }
}
