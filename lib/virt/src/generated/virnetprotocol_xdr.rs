// GENERATED CODE
//
// Generated from /home/f1shl3gs/Workspaces/clion/vertex/target/debug/build/virt-84ca5d2b339de854/out/virnetprotocol.x by xdrgen.
//
// DO NOT EDIT

pub const VIR_NET_MESSAGE_HEADER_MAX: i64 = 24i64;

pub const VIR_NET_MESSAGE_HEADER_XDR_LEN: i64 = 4i64;

pub const VIR_NET_MESSAGE_INITIAL: i64 = 65536i64;

pub const VIR_NET_MESSAGE_LEGACY_PAYLOAD_MAX: i64 = 262120i64;

pub const VIR_NET_MESSAGE_LEN_MAX: i64 = 4i64;

pub const VIR_NET_MESSAGE_MAX: i64 = 16777216i64;

pub const VIR_NET_MESSAGE_NUM_FDS_MAX: i64 = 32i64;

pub const VIR_NET_MESSAGE_PAYLOAD_MAX: i64 = 16777192i64;

pub const VIR_NET_MESSAGE_STRING_MAX: i64 = 4194304i64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct virNetMessageError {
    pub code: i32,
    pub domain: i32,
    pub message: virNetMessageString,
    pub level: i32,
    pub dom: virNetMessageDomain,
    pub str1: virNetMessageString,
    pub str2: virNetMessageString,
    pub str3: virNetMessageString,
    pub int1: i32,
    pub int2: i32,
    pub net: virNetMessageNetwork,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct virNetMessageHeader {
    pub prog: u32,
    pub vers: u32,
    pub proc_: i32,
    pub type_: virNetMessageType,
    pub serial: u32,
    pub status: virNetMessageStatus,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct virNetMessageNonnullDomain {
    pub name: virNetMessageNonnullString,
    pub uuid: virNetMessageUUID,
    pub id: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct virNetMessageNonnullNetwork {
    pub name: virNetMessageNonnullString,
    pub uuid: virNetMessageUUID,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct virNetMessageNonnullString(pub String);

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum virNetMessageStatus {
    VIR_NET_OK = 0isize,
    VIR_NET_ERROR = 1isize,
    VIR_NET_CONTINUE = 2isize,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum virNetMessageType {
    VIR_NET_CALL = 0isize,
    VIR_NET_REPLY = 1isize,
    VIR_NET_MESSAGE = 2isize,
    VIR_NET_STREAM = 3isize,
    VIR_NET_CALL_WITH_FDS = 4isize,
    VIR_NET_REPLY_WITH_FDS = 5isize,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct virNetMessageUUID(pub [u8; 16i64 as usize]);

pub type virNetMessageDomain = Option<Box<virNetMessageNonnullDomain>>;

pub type virNetMessageNetwork = Option<Box<virNetMessageNonnullNetwork>>;

pub type virNetMessageString = Option<virNetMessageNonnullString>;

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for virNetMessageError {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.code.pack(out)?
            + self.domain.pack(out)?
            + self.message.pack(out)?
            + self.level.pack(out)?
            + self.dom.pack(out)?
            + self.str1.pack(out)?
            + self.str2.pack(out)?
            + self.str3.pack(out)?
            + self.int1.pack(out)?
            + self.int2.pack(out)?
            + self.net.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for virNetMessageHeader {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.prog.pack(out)?
            + self.vers.pack(out)?
            + self.proc_.pack(out)?
            + self.type_.pack(out)?
            + self.serial.pack(out)?
            + self.status.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for virNetMessageNonnullDomain {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.name.pack(out)? + self.uuid.pack(out)? + self.id.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for virNetMessageNonnullNetwork {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.name.pack(out)? + self.uuid.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for virNetMessageNonnullString {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_string(
            &self.0,
            Some(VIR_NET_MESSAGE_STRING_MAX as usize),
            out,
        )?)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for virNetMessageStatus {
    #[inline]
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok((*self as i32).pack(out)?)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for virNetMessageType {
    #[inline]
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok((*self as i32).pack(out)?)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for virNetMessageUUID {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_opaque_array(
            &self.0[..],
            self.0.len(),
            out,
        )?)
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for virNetMessageError {
    fn unpack(input: &mut In) -> xdr_codec::Result<(virNetMessageError, usize)> {
        let mut sz = 0;
        Ok((
            virNetMessageError {
                code: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                domain: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                message: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                level: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                str1: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                str2: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                str3: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                int1: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                int2: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                net: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for virNetMessageHeader {
    fn unpack(input: &mut In) -> xdr_codec::Result<(virNetMessageHeader, usize)> {
        let mut sz = 0;
        Ok((
            virNetMessageHeader {
                prog: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                vers: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                proc_: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                type_: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                serial: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                status: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for virNetMessageNonnullDomain {
    fn unpack(input: &mut In) -> xdr_codec::Result<(virNetMessageNonnullDomain, usize)> {
        let mut sz = 0;
        Ok((
            virNetMessageNonnullDomain {
                name: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                uuid: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                id: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for virNetMessageNonnullNetwork {
    fn unpack(input: &mut In) -> xdr_codec::Result<(virNetMessageNonnullNetwork, usize)> {
        let mut sz = 0;
        Ok((
            virNetMessageNonnullNetwork {
                name: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                uuid: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for virNetMessageNonnullString {
    fn unpack(input: &mut In) -> xdr_codec::Result<(virNetMessageNonnullString, usize)> {
        let mut sz = 0;
        Ok((
            {
                let (v, usz) =
                    xdr_codec::unpack_string(input, Some(VIR_NET_MESSAGE_STRING_MAX as usize))?;
                sz = usz;
                virNetMessageNonnullString(v)
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for virNetMessageStatus {
    #[inline]
    fn unpack(input: &mut In) -> xdr_codec::Result<(virNetMessageStatus, usize)> {
        let mut sz = 0;
        Ok((
            {
                let (e, esz): (i32, _) = xdr_codec::Unpack::unpack(input)?;
                sz += esz;
                match e {
                    x if x == virNetMessageStatus::VIR_NET_OK as i32 => {
                        virNetMessageStatus::VIR_NET_OK
                    }
                    x if x == virNetMessageStatus::VIR_NET_ERROR as i32 => {
                        virNetMessageStatus::VIR_NET_ERROR
                    }
                    x if x == virNetMessageStatus::VIR_NET_CONTINUE as i32 => {
                        virNetMessageStatus::VIR_NET_CONTINUE
                    }
                    e => return Err(xdr_codec::Error::invalidenum(e)),
                }
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for virNetMessageType {
    #[inline]
    fn unpack(input: &mut In) -> xdr_codec::Result<(virNetMessageType, usize)> {
        let mut sz = 0;
        Ok((
            {
                let (e, esz): (i32, _) = xdr_codec::Unpack::unpack(input)?;
                sz += esz;
                match e {
                    x if x == virNetMessageType::VIR_NET_CALL as i32 => {
                        virNetMessageType::VIR_NET_CALL
                    }
                    x if x == virNetMessageType::VIR_NET_REPLY as i32 => {
                        virNetMessageType::VIR_NET_REPLY
                    }
                    x if x == virNetMessageType::VIR_NET_MESSAGE as i32 => {
                        virNetMessageType::VIR_NET_MESSAGE
                    }
                    x if x == virNetMessageType::VIR_NET_STREAM as i32 => {
                        virNetMessageType::VIR_NET_STREAM
                    }
                    x if x == virNetMessageType::VIR_NET_CALL_WITH_FDS as i32 => {
                        virNetMessageType::VIR_NET_CALL_WITH_FDS
                    }
                    x if x == virNetMessageType::VIR_NET_REPLY_WITH_FDS as i32 => {
                        virNetMessageType::VIR_NET_REPLY_WITH_FDS
                    }
                    e => return Err(xdr_codec::Error::invalidenum(e)),
                }
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for virNetMessageUUID {
    fn unpack(input: &mut In) -> xdr_codec::Result<(virNetMessageUUID, usize)> {
        let mut sz = 0;
        Ok((
            {
                let (v, usz) = {
                    let mut buf: [u8; 16i64 as usize] = [0; 16];
                    let sz = xdr_codec::unpack_opaque_array(input, &mut buf[..], 16i64 as usize)?;
                    (buf, sz)
                };
                sz = usz;
                virNetMessageUUID(v)
            },
            sz,
        ))
    }
}
