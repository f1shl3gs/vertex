use std::io::{Read, Write};

use super::constants::VIR_NET_MESSAGE_STRING_MAX;
use super::{impl_procedure, unpack_string, Domain, Pack, Result, Unpack};

pub struct DomainGetXmlDescRequest<'a> {
    pub domain: &'a Domain,
    pub flags: u32,
}

impl_procedure!(DomainGetXmlDescRequest<'_>, REMOTE_PROC_DOMAIN_GET_XML_DESC);

impl<W: Write> Pack<W> for DomainGetXmlDescRequest<'_> {
    fn pack(&self, w: &mut W) -> Result<usize> {
        Ok(self.domain.pack(w)? + self.flags.pack(w)?)
    }
}

pub struct DomainGetXmlDescResponse {
    pub data: String,
}

impl<R: Read> Unpack<R> for DomainGetXmlDescResponse {
    fn unpack(r: &mut R) -> Result<(Self, usize)> {
        let (data, sz) = unpack_string(r, VIR_NET_MESSAGE_STRING_MAX)?;

        Ok((DomainGetXmlDescResponse { data }, sz))
    }
}
