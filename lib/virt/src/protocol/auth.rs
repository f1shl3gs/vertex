use std::io::{Read, Write};

use super::{impl_procedure, Pack, Unpack};
use crate::protocol::unpack_flex;

const REMOTE_AUTH_TYPE_LIST_MAX: usize = 20;

pub struct AuthListRequest {}

impl_procedure!(AuthListRequest, REMOTE_PROC_AUTH_LIST);

impl<W: Write> Pack<W> for AuthListRequest {
    fn pack(&self, _w: &mut W) -> crate::protocol::Result<usize> {
        Ok(0)
    }
}

pub enum AuthType {
    None,
    SASL,
    POLKIT,
}

impl From<i64> for AuthType {
    fn from(value: i64) -> Self {
        match value {
            0 => AuthType::None,
            1 => AuthType::SASL,
            2 => AuthType::POLKIT,
            _ => unreachable!("unknown auth type"),
        }
    }
}

pub struct AuthListResponse {
    pub types: Vec<AuthType>,
}

impl<R: Read> Unpack<R> for AuthListResponse {
    fn unpack(r: &mut R) -> crate::protocol::Result<(Self, usize)> {
        let (ret, sz): (Vec<i64>, usize) = unpack_flex(r, REMOTE_AUTH_TYPE_LIST_MAX)?;

        Ok((
            Self {
                types: ret.into_iter().map(AuthType::from).collect(),
            },
            sz,
        ))
    }
}
