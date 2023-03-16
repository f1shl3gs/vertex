use std::borrow::{Borrow, Cow};
use std::convert;
use std::io::{self, Read, Write};
use std::ops::Deref;

use crate::protocol::constants::{
    MESSAGE_STATUS_OK, MESSAGE_TYPE_CALL, VIR_NET_MESSAGE_STRING_MAX,
};
use crate::protocol::{pack_flex, Error, Pack, ReadExt, Result, Unpack, WriteExt};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct MessageHeader {
    pub program: u32,
    pub version: u32,
    pub procedure: i32,
    pub typ: i32,
    pub serial: u32,
    pub status: i32,
}

impl MessageHeader {
    pub fn success(&self) -> bool {
        self.status == MESSAGE_STATUS_OK
    }
}

impl Default for MessageHeader {
    fn default() -> Self {
        MessageHeader {
            program: 0x20008086,
            version: 1,
            procedure: 0,
            typ: MESSAGE_TYPE_CALL,
            serial: 0,
            status: MESSAGE_STATUS_OK,
        }
    }
}

impl MessageHeader {
    pub(crate) fn pack<W: Write>(&self, w: &mut W) -> io::Result<usize> {
        w.write_u32(self.program)?;
        w.write_u32(self.version)?;
        w.write_i32(self.procedure)?;
        w.write_i32(self.typ)?;
        w.write_u32(self.serial)?;
        w.write_i32(self.status as i32)?;

        Ok(24)
    }
}

impl<R: Read> Unpack<R> for MessageHeader {
    fn unpack(r: &mut R) -> Result<(Self, usize)> {
        let program = r.read_u32()?;
        let version = r.read_u32()?;
        let procedure = r.read_i32()?;
        let typ = r.read_i32()?;
        let serial = r.read_u32()?;
        let status = r.read_i32()?;

        Ok((
            Self {
                program,
                version,
                procedure,
                typ,
                serial,
                status,
            },
            24,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct Domain {
    name: String,
    uuid: [u8; 16],
    pub id: i32,
}

impl Domain {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn uuid(&self) -> String {
        to_uuid(self.uuid)
    }
}

const LOWER: [u8; 16] = [
    b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'a', b'b', b'c', b'd', b'e', b'f',
];

fn to_uuid(src: [u8; 16]) -> String {
    let groups = [(0, 8), (9, 13), (14, 18), (19, 23), (24, 36)];
    let mut dst = [0; 36];

    let mut group_idx = 0;
    let mut i = 0;
    while group_idx < 5 {
        let (start, end) = groups[group_idx];
        let mut j = start;
        while j < end {
            let x = src[i];
            i += 1;

            dst[j] = LOWER[(x >> 4) as usize];
            dst[j + 1] = LOWER[(x & 0x0f) as usize];
            j += 2;
        }
        if group_idx < 4 {
            dst[end] = b'-';
        }
        group_idx += 1;
    }

    String::from_utf8_lossy(&dst).to_string()
}

impl<W: Write> Pack<W> for u8 {
    #[inline]
    fn pack(&self, w: &mut W) -> Result<usize> {
        w.write_u32(*self as u32).map_err(Error::from).map(|_| 4)
    }
}

impl<W: Write> Pack<W> for str {
    #[inline]
    fn pack(&self, w: &mut W) -> Result<usize> {
        Opaque::borrowed(self.as_bytes()).pack(w)
    }
}

impl<W: Write> Pack<W> for Domain {
    fn pack(&self, w: &mut W) -> Result<usize> {
        Ok(self.name.pack(w)? + pack_flex(&self.uuid, Some(16), w)? + self.id.pack(w)?)
    }
}

impl<R: Read> Unpack<R> for Domain {
    fn unpack(r: &mut R) -> Result<(Self, usize)> {
        let (name, size) = Unpack::unpack(r)?;
        let mut uuid = [0u8; 16];
        r.read_exact(&mut uuid)?;
        let id = r.read_i32()?;

        Ok((Self { name, uuid, id }, size + 16 + 4))
    }
}

impl<R: Read> Unpack<R> for u8 {
    #[inline]
    fn unpack(r: &mut R) -> Result<(Self, usize)> {
        r.read_u32().map_err(Error::from).map(|v| (v as u8, 4))
    }
}

impl<R: Read> Unpack<R> for i32 {
    #[inline]
    fn unpack(input: &mut R) -> Result<(Self, usize)> {
        input.read_i32().map_err(Error::from).map(|v| (v, 4))
    }
}

impl<R: Read> Unpack<R> for u64 {
    #[inline]
    fn unpack(r: &mut R) -> Result<(Self, usize)> {
        r.read_u64().map_err(Error::from).map(|v| (v, 8))
    }
}

#[derive(Debug)]
pub struct Network {
    pub name: String,
    pub uuid: [u8; 16],
}

impl<R: Read> Unpack<R> for Network {
    fn unpack(r: &mut R) -> Result<(Self, usize)> {
        let (name, size) = unpack_string(r, VIR_NET_MESSAGE_STRING_MAX)?;
        let mut uuid = [0u8; 16];
        r.read_exact(&mut uuid)?;

        Ok((Self { name, uuid }, size + 16))
    }
}

#[derive(Debug)]
pub struct MessageError {
    pub code: i32,
    pub domain: i32,
    pub message: Option<String>,
    pub level: i32,
    pub dom: Option<Domain>,
    pub str1: Option<String>,
    pub str2: Option<String>,
    pub str3: Option<String>,
    pub int1: i32,
    pub int2: i32,
    pub net: Option<Network>,
}

impl MessageError {
    pub fn unpack<R: ReadExt>(r: &mut R) -> Result<(Self, usize)> {
        let mut sz = 0;
        Ok((
            Self {
                code: {
                    let (v, fsz) = Unpack::unpack(r)?;
                    sz += fsz;
                    v
                },
                domain: {
                    let (v, fsz) = Unpack::unpack(r)?;
                    sz += fsz;
                    v
                },
                message: {
                    let (v, fsz) = Unpack::unpack(r)?;
                    sz += fsz;
                    v
                },
                level: {
                    let (v, fsz) = Unpack::unpack(r)?;
                    sz += fsz;
                    v
                },
                dom: {
                    let (v, fsz) = Unpack::unpack(r)?;
                    sz += fsz;
                    v
                },
                str1: {
                    let (v, fsz) = Unpack::unpack(r)?;
                    sz += fsz;
                    v
                },
                str2: {
                    let (v, fsz) = Unpack::unpack(r)?;
                    sz += fsz;
                    v
                },
                str3: {
                    let (v, fsz) = Unpack::unpack(r)?;
                    sz += fsz;
                    v
                },
                int1: {
                    let (v, fsz) = Unpack::unpack(r)?;
                    sz += fsz;
                    v
                },
                int2: {
                    let (v, fsz) = Unpack::unpack(r)?;
                    sz += fsz;
                    v
                },
                net: {
                    let (v, fsz) = Unpack::unpack(r)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<R: Read> Unpack<R> for String {
    #[inline]
    fn unpack(r: &mut R) -> Result<(Self, usize)> {
        unpack_string(r, VIR_NET_MESSAGE_STRING_MAX)
    }
}

/// Wrapper for XDR opaque data.
///
/// In XDR terms, "opaque data" is a plain array of bytes, packed as tightly as possible, and then
/// padded to a 4 byte offset. This is different from an array of bytes, where each byte would be
/// padded to 4 bytes when emitted into the array.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Opaque<'a>(pub Cow<'a, [u8]>);

impl<'a> Opaque<'a> {
    pub fn owned(v: Vec<u8>) -> Opaque<'a> {
        Opaque(Cow::Owned(v))
    }
    pub fn borrowed(v: &'a [u8]) -> Opaque<'a> {
        Opaque(Cow::Borrowed(v))
    }

    fn pack<W: Write>(&self, w: &mut W) -> Result<usize> {
        let data: &[u8] = self.0.borrow();

        if data.len() > u32::MAX as usize {
            return Err(Error::InvalidLen(data.len()));
        }

        w.write_u32(data.len() as u32)?;
        w.write_all(data)?;

        let mut size = 8 + data.len();

        let p = padding(size);
        if p.len() > 0 {
            w.write_all(p)?;
            size += p.len();
        }

        Ok(size)
    }
}

impl<'a> Deref for Opaque<'a> {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        self.0.deref()
    }
}

impl<'a> From<&'a [u8]> for Opaque<'a> {
    fn from(v: &'a [u8]) -> Self {
        Opaque::borrowed(v)
    }
}

/// Pack a dynamically sized opaque array, with size limit check.
///
/// This packs an array of packable objects, and also applies an optional size limit.
#[inline]
pub fn pack_opaque_flex<Out: Write>(
    val: &[u8],
    maxsz: Option<usize>,
    out: &mut Out,
) -> Result<usize> {
    if maxsz.map_or(false, |m| val.len() > m) {
        return Err(Error::InvalidLen(maxsz.unwrap()));
    }

    Opaque::borrowed(val).pack(out)
}

/// Pack a string with size limit check.
#[inline]
pub fn pack_string<W: Write>(val: &str, max_size: Option<usize>, w: &mut W) -> Result<usize> {
    pack_opaque_flex(val.as_bytes(), max_size, w)
}

/// Unpack a (perhaps) length-limited opaque array
///
/// Unpack an XDR encoded array of bytes, with an optional maximum length.
pub fn unpack_opaque_flex<R: Read>(r: &mut R, max_sz: usize) -> Result<(Vec<u8>, usize)> {
    let (elems, mut sz) = Unpack::unpack(r)?;

    if elems > max_sz {
        return Err(Error::InvalidLen(max_sz));
    }

    let mut out = Vec::with_capacity(elems);

    sz += r.take(elems as u64).read_to_end(&mut out)?;

    let p = padding(sz);
    for _ in 0..p.len() {
        let _ = r.read_u8()?;
    }
    sz += p.len();

    Ok((out, sz))
}

/// Unpack (perhaps) length-limited string
pub fn unpack_string<R: Read>(r: &mut R, max_size: usize) -> Result<(String, usize)> {
    let (v, sz) = unpack_opaque_flex(r, max_size)?;

    String::from_utf8(v).map_err(Error::from).map(|s| (s, sz))
}

static PADDING: [u8; 4] = [0; 4];

/// Compute XDR padding.
///
/// Return slice of zero padding needed to bring `sz` up to a multiple of 4. If no padding is needed,
/// it will be a zero-sized slice.
#[inline]
pub fn padding(sz: usize) -> &'static [u8] {
    &PADDING[..(4 - (sz % 4)) % 4]
}

impl<R: Read> Unpack<R> for i64 {
    fn unpack(r: &mut R) -> Result<(Self, usize)> {
        let v = r.read_i64().map_err(Error::Io)?;
        Ok((v, 8))
    }
}

impl<W: Write> Pack<W> for i32 {
    fn pack(&self, w: &mut W) -> Result<usize> {
        w.write_i32(*self)?;
        Ok(4)
    }
}

impl<W: Write> Pack<W> for String {
    #[inline]
    fn pack(&self, w: &mut W) -> Result<usize> {
        pack_string(self, Some(VIR_NET_MESSAGE_STRING_MAX), w)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::assert_pack;
    use std::io::Cursor;

    #[test]
    fn string() {
        let max_size = 64;
        let input = "abcdefghijklmnopqrstuvwxyz0123456789";
        let want = &[
            0x00, 0x00, 0x00, 0x24, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6a,
            0x6b, 0x6c, 0x6d, 0x6e, 0x6f, 0x70, 0x71, 0x72, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78,
            0x79, 0x7a, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39,
        ];

        let mut buf = Vec::new();
        pack_string(input, Some(max_size), &mut buf).unwrap();
        assert_eq!(buf, want);

        let mut c = Cursor::new(buf);
        let (n, _) = unpack_string(&mut c, max_size).unwrap();
        assert_eq!(n, input);
    }
}
