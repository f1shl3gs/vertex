use std::io::{BufRead, Cursor};

use bytes::Buf;

use super::ipfix::{DataRecord, OptionsDataRecord};
use super::template::{Field, TemplateRecord};
use crate::common::read::ReadExt;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(std::io::Error),

    #[error("datagram is too short")]
    DatagramTooShort,

    #[error("no field in template")]
    NoFieldInTemplate,

    #[error("template {template_id} not found with observation domain id {observation_domain_id}")]
    TemplateNotFound {
        observation_domain_id: u32,
        template_id: u16,
    },

    #[error("unknown flow set id {0}")]
    UnknownFlowSetID(u16),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

#[derive(Debug)]
pub struct DataField<'a> {
    pub typ: u16,
    // https://datatracker.ietf.org/doc/html/rfc7011#section-6.1
    pub data: &'a [u8],
}

pub fn decode_template_records(
    buf: &mut Cursor<&[u8]>,
    version: u16,
) -> Result<Vec<TemplateRecord>, Error> {
    let mut templates = Vec::new();

    while buf.remaining() >= 4 {
        let template_id = buf.read_u16()?;
        let field_count = buf.read_u16()?;
        if field_count == 0 {
            return Err(Error::NoFieldInTemplate);
        }

        let mut fields = vec![];
        for _ in 0..field_count {
            let mut field_type = buf.read_u16()?;
            let field_length = buf.read_u16()?;

            let pen = if version == 10 && field_type & 0x8000 != 0 {
                field_type ^= 0x8000;
                let pen = buf.read_u32()?;
                Some(pen)
            } else {
                None
            };

            fields.push(Field {
                typ: field_type,
                length: field_length,
                pen,
            })
        }

        templates.push(TemplateRecord {
            id: template_id,
            fields,
        })
    }

    Ok(templates)
}

pub fn decode_data_records<'a>(
    buf: &mut Cursor<&'a [u8]>,
    fields: &[Field],
) -> Result<Vec<DataRecord<'a>>, Error> {
    let mut records = Vec::new();

    let length = fields
        .iter()
        .filter(|f| f.length != 0xffff)
        .map(|field| field.length)
        .sum::<u16>();

    while buf.remaining() > length as usize {
        let mut data_fields = Vec::with_capacity(fields.len());
        for field in fields {
            if field.length == 0xffff {
                unimplemented!()
            }

            let start = buf.position() as usize;
            let data = &buf.get_ref()[start..start + field.length as usize];
            buf.consume(field.length as usize);
            data_fields.push(DataField {
                typ: field.typ,
                data,
            })
        }

        records.push(DataRecord {
            fields: data_fields,
        })
    }

    Ok(records)
}

pub fn decode_options_data_records<'a>(
    buf: &mut Cursor<&'a [u8]>,
    scopes: &[Field],
    options: &[Field],
) -> Result<Vec<OptionsDataRecord<'a>>, Error> {
    let mut records = Vec::new();

    let length = options
        .iter()
        .chain(scopes.iter())
        .map(|field| field.length)
        .sum::<u16>();

    while buf.remaining() > length as usize {
        let mut scopes_fields = Vec::with_capacity(scopes.len());
        for field in scopes {
            if field.length == 0xffff {
                unimplemented!()
            }

            let start = buf.position() as usize;
            let data = &buf.get_ref()[start..start + field.length as usize];
            buf.consume(field.length as usize);
            scopes_fields.push(DataField {
                typ: field.typ,
                data,
            })
        }

        let mut options_fields = Vec::with_capacity(options.len());
        for field in options {
            if field.length == 0xffff {
                unimplemented!()
            }

            let start = buf.position() as usize;
            let data = &buf.get_ref()[start..start + field.length as usize];
            buf.consume(field.length as usize);
            options_fields.push(DataField {
                typ: field.typ,
                data,
            })
        }

        records.push(OptionsDataRecord {
            scopes: scopes_fields,
            options: options_fields,
        })
    }

    Ok(records)
}
