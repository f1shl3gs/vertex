use std::io::{Read, Result};

pub trait ReadExt: Read {
    fn read_i8(&mut self) -> Result<i8> {
        let mut buf = [0; 1];
        self.read_exact(&mut buf)?;
        Ok(buf[0] as i8)
    }

    fn read_i16(&mut self) -> Result<i16> {
        let mut buf = [0; 2];
        self.read_exact(&mut buf)?;
        Ok(i16::from_be_bytes(buf))
    }

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

    fn read_f64(&mut self) -> Result<f64> {
        let mut buf = [0; 8];
        self.read_exact(&mut buf)?;
        Ok(f64::from_be_bytes(buf))
    }
}

impl<T> ReadExt for T where T: Read {}
