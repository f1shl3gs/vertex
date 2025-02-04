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
