//! RFC 4506 - XDR: External Data Representation Standard
//!
//! https://datatracker.ietf.org/doc/html/rfc4506

use std::io::{Read, Result};

pub trait XDRReader: Read {
    fn read_i32(&mut self) -> Result<i32> {
        let mut buf = [0; 4];
        self.read_exact(&mut buf)?;
        Ok(i32::from_be_bytes(buf))
    }

    fn read_i64(&mut self) -> Result<i64> {
        let mut buf = [0; 8];
        self.read_exact(&mut buf)?;
        Ok(i64::from_be_bytes(buf))
    }

    fn read_u8(&mut self) -> Result<u8> {
        let mut buf = [0; 1];
        self.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    fn read_u16(&mut self) -> Result<u16> {
        let mut buf = [0; 2];
        self.read_exact(&mut buf)?;
        Ok(u16::from_be_bytes(buf))
    }

    fn read_u32(&mut self) -> Result<u32> {
        let mut buf = [0; 4];
        self.read_exact(&mut buf)?;
        Ok(u32::from_be_bytes(buf))
    }

    fn read_u64(&mut self) -> Result<u64> {
        let mut buf = [0; 8];
        self.read_exact(&mut buf)?;
        Ok(u64::from_be_bytes(buf))
    }

    fn read_f32(&mut self) -> Result<f32> {
        let mut buf = [0; 4];
        self.read_exact(&mut buf)?;
        Ok(f32::from_be_bytes(buf))
    }

    fn read_string(&mut self) -> Result<String> {
        let len = self.read_u32()?;
        let aligned_len = (len + 3) & (!3); // align to 4

        let mut data = vec![0u8; aligned_len as usize];
        self.read_exact(&mut data)?;
        data.truncate(len as usize);

        Ok(unsafe { String::from_utf8_unchecked(data) })
    }
}

impl<T> XDRReader for T where T: Read {}

pub struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl Read for Reader<'_> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let amount = buf.len().min(self.data.len() - self.pos);
        let src = &self.data[self.pos..self.pos + amount];
        buf[..amount].copy_from_slice(src);
        self.pos += amount;
        Ok(amount)
    }
}

impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8]) -> Reader<'a> {
        Reader { data, pos: 0 }
    }

    pub fn take(&'a mut self, len: usize) -> Reader<'a> {
        let start = self.pos;
        self.pos += len;

        Reader {
            data: &self.data[start..self.pos],
            pos: 0,
        }
    }

    #[inline]
    pub fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read() {
        let data = &[0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
        let mut reader = Reader { data, pos: 0 };

        // buf not full
        let mut buf = [0u8; 4];
        let read = reader.read(&mut buf).unwrap();
        assert_eq!(read, 4);
        assert_eq!(buf, [0, 1, 2, 3]);
        assert_eq!(reader.pos, 4);

        // read zero
        let mut buf = [0u8; 0];
        let read = reader.read(&mut buf).unwrap();
        assert_eq!(read, 0);
        assert_eq!(reader.pos, 4);

        // read unfill
        let mut buf = [0u8; 16];
        let read = reader.read(&mut buf).unwrap();
        assert_eq!(read, 12);
        assert_eq!(reader.pos, 16);
    }
}
