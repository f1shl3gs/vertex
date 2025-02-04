#![allow(dead_code)]

//! Cisco Systems NetFlow Services Export Version 9
//!
//! https://datatracker.ietf.org/doc/html/rfc3954
//! https://www.cisco.com/en/US/technologies/tk648/tk362/technologies_white_paper09186a00800a3db9.html

use std::io::Cursor;

use bytes::Buf;
use xdr::XDRReader;

use super::decode::{
    decode_data_records, decode_options_data_records, decode_template_records, Error,
};
use super::ipfix::{DataRecord, OptionsDataRecord};
use super::template::{Field, Template, TemplateRecord, TemplateSystem};

#[derive(Debug)]
pub struct OptionsTemplateRecord {
    pub id: u16,
    pub scope_length: u16,
    pub option_length: u16,
    pub scopes: Vec<Field>,
    pub options: Vec<Field>,
}

#[derive(Debug)]
pub enum FlowSet<'a> {
    Template {
        id: u16,
        records: Vec<TemplateRecord>,
    },
    Data {
        template_id: u16,
        length: u16,
        records: Vec<DataRecord<'a>>,
    },
    OptionsTemplate {
        id: u16,
        records: Vec<OptionsTemplateRecord>,
    },
    OptionsData {
        template_id: u16,
        records: Vec<OptionsDataRecord<'a>>,
    },
}

#[derive(Debug)]
pub struct NetFlow<'a> {
    pub version: u16,
    // The total number of records in the Export Packet, which is the
    // sum of Options FlowSet records, Template FlowSet records, and
    // Data FlowSet records.
    pub count: u16,
    pub system_uptime: u32,
    pub unix_seconds: u32,
    pub sequence_number: u32,
    pub source_id: u32,

    pub flow_sets: Vec<FlowSet<'a>>,
}

impl<'a> NetFlow<'a> {
    pub fn decode<T: TemplateSystem>(
        data: &'a [u8],
        templates: &mut T,
    ) -> Result<NetFlow<'a>, Error> {
        let mut buf = Cursor::new(data);

        let version = buf.read_u16()?;
        let count = buf.read_u16()?;
        let system_uptime = buf.read_u32()?;
        let unix_seconds = buf.read_u32()?;
        let sequence_number = buf.read_u32()?;
        let source_id = buf.read_u32()?;

        let mut flow_sets = Vec::with_capacity(count as usize);
        while buf.remaining() > 0 {
            flow_sets.push(decode_flow_set(&mut buf, source_id, templates)?);
        }

        Ok(NetFlow {
            version,
            count,
            system_uptime,
            unix_seconds,
            sequence_number,
            source_id,
            flow_sets,
        })
    }
}

fn decode_flow_set<'a, T: TemplateSystem>(
    buf: &mut Cursor<&'a [u8]>,
    odid: u32,
    templates: &mut T,
) -> Result<FlowSet<'a>, Error> {
    let id = buf.read_u16()?;
    let length = buf.read_u16()?;

    let payload = buf.get_ref();
    let start = buf.position() as usize;
    let end = start + length as usize - 4; // 4 is the length of id and length
    let mut set_buf = Cursor::new(&payload[start..end]);
    buf.set_position(end as u64);

    let set = if id == 0 {
        // template record
        let records = decode_template_records(&mut set_buf, 9)?;
        for record in &records {
            println!("add template {odid}/{id}");

            templates.add(
                9,
                odid,
                record.id,
                Template::Basic {
                    fields: record.fields.clone(),
                },
            );
        }

        FlowSet::Template { id, records }
    } else if id == 1 {
        // options template record
        let records = decode_options_template_records(&mut set_buf)?;
        for record in &records {
            templates.add(
                9,
                odid,
                record.id,
                Template::Options {
                    scopes: record.scopes.clone(),
                    options: record.options.clone(),
                },
            );
        }

        FlowSet::OptionsTemplate { id, records }
    } else if id >= 256 {
        // data record, it might be basic template or options template
        let template = templates.get(9, odid, id).ok_or(Error::TemplateNotFound {
            observation_domain_id: odid,
            template_id: id,
        })?;

        match template {
            Template::Basic { fields } => FlowSet::Data {
                template_id: id,
                length,
                records: decode_data_records(&mut set_buf, fields)?,
            },
            Template::Options { scopes, options } => FlowSet::OptionsData {
                template_id: id,
                records: decode_options_data_records(&mut set_buf, scopes, options)?,
            },
        }
    } else {
        return Err(Error::UnknownFlowSetID(id));
    };

    Ok(set)
}

fn decode_options_template_records(
    buf: &mut Cursor<&[u8]>,
) -> Result<Vec<OptionsTemplateRecord>, Error> {
    let mut records = vec![];

    while buf.remaining() >= 4 {
        let id = buf.read_u16()?;
        let scope_length = buf.read_u16()?;
        let option_length = buf.read_u16()?;

        let mut scopes = vec![];
        for _ in 0..scope_length / 4 {
            let typ = buf.read_u16()?;
            let length = buf.read_u16()?;

            scopes.push(Field {
                typ,
                length,
                pen: None,
            })
        }

        let mut options = vec![];
        for _ in 0..option_length / 4 {
            let typ = buf.read_u16()?;
            let length = buf.read_u16()?;

            options.push(Field {
                typ,
                length,
                pen: None,
            })
        }

        records.push(OptionsTemplateRecord {
            id,
            scope_length,
            option_length,
            scopes,
            options,
        })
    }

    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::netflow::template::BasicTemplateSystem;

    #[test]
    fn decode() {
        let mut templates = BasicTemplateSystem::default();

        let data = [
            0x00, 0x09, 0x00, 0x01, 0xb3, 0xbf, 0xf6, 0x83, 0x61, 0x8a, 0xa3, 0xa8, 0x32, 0x01,
            0xee, 0x98, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x64, 0x01, 0x04, 0x00, 0x17,
            0x00, 0x02, 0x00, 0x04, 0x00, 0x01, 0x00, 0x04, 0x00, 0x08, 0x00, 0x04, 0x00, 0x0c,
            0x00, 0x04, 0x00, 0x0a, 0x00, 0x04, 0x00, 0x0e, 0x00, 0x04, 0x00, 0x15, 0x00, 0x04,
            0x00, 0x16, 0x00, 0x04, 0x00, 0x07, 0x00, 0x02, 0x00, 0x0b, 0x00, 0x02, 0x00, 0x10,
            0x00, 0x04, 0x00, 0x11, 0x00, 0x04, 0x00, 0x12, 0x00, 0x04, 0x00, 0x09, 0x00, 0x01,
            0x00, 0x0d, 0x00, 0x01, 0x00, 0x04, 0x00, 0x01, 0x00, 0x06, 0x00, 0x01, 0x00, 0x05,
            0x00, 0x01, 0x00, 0x3d, 0x00, 0x01, 0x00, 0x59, 0x00, 0x01, 0x00, 0x30, 0x00, 0x02,
            0x00, 0xea, 0x00, 0x04, 0x00, 0xeb, 0x00, 0x04,
        ];
        let netflow = NetFlow::decode(&data, &mut templates).unwrap();
        println!("{:#?}", netflow);

        let data = [
            0x00, 0x09, 0x00, 0x15, 0xb3, 0xbf, 0xf6, 0x83, 0x61, 0x8a, 0xa3, 0xa8, 0x32, 0x01,
            0xee, 0x9c, 0x00, 0x00, 0x01, 0x00, 0x01, 0x04, 0x05, 0x5c, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x00, 0x05, 0xdc, 0xc6, 0x26, 0x78, 0xde, 0x58, 0x79, 0xd9, 0xd0, 0x00, 0x00,
            0x01, 0x62, 0x00, 0x00, 0x01, 0x30, 0xb3, 0xbf, 0xe6, 0xf9, 0xb3, 0xbf, 0xe6, 0xf9,
            0x01, 0xbb, 0x3b, 0x50, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xfc, 0xdf,
            0x00, 0x00, 0x18, 0x0e, 0x06, 0x10, 0x00, 0x00, 0x40, 0x00, 0x01, 0x60, 0x00, 0x00,
            0x02, 0x60, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x0b, 0xb8, 0x6d,
            0x47, 0xa2, 0xc4, 0x5b, 0xad, 0x61, 0xe0, 0x00, 0x00, 0x01, 0x61, 0x00, 0x00, 0x01,
            0x30, 0xb3, 0xbf, 0xe8, 0x1c, 0xb3, 0xbf, 0xe6, 0xf9, 0x01, 0xbb, 0x7b, 0x99, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xfc, 0xdf, 0x00, 0x00, 0x18, 0x0d, 0x06,
            0x10, 0x48, 0x00, 0x40, 0x00, 0x01, 0x60, 0x00, 0x00, 0x02, 0x60, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x05, 0xdc, 0xc6, 0x26, 0x78, 0xd3, 0x5b, 0xa5,
            0xd2, 0xee, 0x00, 0x00, 0x01, 0x62, 0x00, 0x00, 0x01, 0x75, 0xb3, 0xbf, 0xe6, 0xfc,
            0xb3, 0xbf, 0xe6, 0xfc, 0x00, 0x50, 0x8f, 0xb8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0xc2, 0x95, 0xae, 0x3b, 0x18, 0x0e, 0x06, 0x10, 0x00, 0x00, 0x40, 0x00,
            0x01, 0x60, 0x00, 0x00, 0x02, 0x60, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00,
            0x00, 0x05, 0xdc, 0x5f, 0x64, 0x56, 0x42, 0x5b, 0xa9, 0x1a, 0xbe, 0x00, 0x00, 0x01,
            0x61, 0x00, 0x00, 0x01, 0x31, 0xb3, 0xbf, 0xe6, 0xfc, 0xb3, 0xbf, 0xe6, 0xfc, 0x00,
            0x50, 0xbf, 0xc3, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xfc, 0xdf, 0x00,
            0x00, 0x18, 0x0e, 0x06, 0x10, 0x28, 0x00, 0x40, 0x00, 0x01, 0x60, 0x00, 0x00, 0x02,
            0x60, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x05, 0xdc, 0xc6, 0x26,
            0x78, 0xc6, 0x5b, 0xab, 0x33, 0x34, 0x00, 0x00, 0x01, 0x62, 0x00, 0x00, 0x01, 0x31,
            0xb3, 0xbf, 0xe6, 0xfc, 0xb3, 0xbf, 0xe6, 0xfc, 0x01, 0xbb, 0xf9, 0xd5, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xfc, 0xdf, 0x00, 0x00, 0x18, 0x0e, 0x06, 0x10,
            0x00, 0x00, 0x40, 0x00, 0x01, 0x60, 0x00, 0x00, 0x02, 0x60, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x01, 0x00, 0x00, 0x05, 0xdc, 0xc6, 0x26, 0x78, 0x83, 0x4e, 0xf2, 0x8c,
            0x81, 0x00, 0x00, 0x01, 0x62, 0x00, 0x00, 0x01, 0x31, 0xb3, 0xbf, 0xe6, 0xfe, 0xb3,
            0xbf, 0xe6, 0xfe, 0x01, 0xbb, 0xb3, 0x60, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0xfc, 0xdf, 0x00, 0x00, 0x18, 0x18, 0x06, 0x10, 0x00, 0x00, 0x40, 0x00, 0x01,
            0x60, 0x00, 0x00, 0x02, 0x60, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
            0x05, 0xdc, 0xc6, 0x26, 0x78, 0xb8, 0x5b, 0xaa, 0xab, 0x01, 0x00, 0x00, 0x01, 0x62,
            0x00, 0x00, 0x01, 0x31, 0xb3, 0xbf, 0xe6, 0xff, 0xb3, 0xbf, 0xe6, 0xff, 0x01, 0xbb,
            0xe5, 0xe5, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xfc, 0xdf, 0x00, 0x00,
            0x18, 0x0e, 0x06, 0x10, 0x00, 0x00, 0x40, 0x00, 0x01, 0x60, 0x00, 0x00, 0x02, 0x60,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x05, 0xdc, 0xc6, 0x26, 0x78,
            0xc5, 0x5b, 0xa5, 0x22, 0x65, 0x00, 0x00, 0x01, 0x62, 0x00, 0x00, 0x01, 0x69, 0xb3,
            0xbf, 0xe7, 0x00, 0xb3, 0xbf, 0xe7, 0x00, 0x01, 0xbb, 0x3c, 0xb4, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0xc2, 0x95, 0xae, 0x31, 0x18, 0x0e, 0x06, 0x10, 0x00,
            0x00, 0x40, 0x00, 0x01, 0x60, 0x00, 0x00, 0x02, 0x60, 0x00, 0x00, 0x02, 0x00, 0x00,
            0x00, 0x01, 0x00, 0x00, 0x05, 0xdc, 0x8f, 0xf4, 0x38, 0x1a, 0x5b, 0xa4, 0xc7, 0x3a,
            0x00, 0x00, 0x01, 0x61, 0x00, 0x00, 0x01, 0x75, 0xb3, 0xbf, 0xe7, 0x01, 0xb3, 0xbf,
            0xe7, 0x01, 0x01, 0xbb, 0x49, 0x7c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0xc2, 0x95, 0xae, 0x3b, 0x17, 0x0e, 0x06, 0x10, 0x28, 0x00, 0x40, 0x00, 0x01, 0x60,
            0x00, 0x00, 0x02, 0x60, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x05,
            0xb0, 0xc7, 0xe8, 0xb2, 0x49, 0x5b, 0xaf, 0x83, 0x0c, 0x00, 0x00, 0x01, 0x61, 0x00,
            0x00, 0x01, 0x30, 0xb3, 0xbf, 0xe7, 0x02, 0xb3, 0xbf, 0xe7, 0x02, 0x01, 0xbb, 0x96,
            0x4a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xfc, 0xdf, 0x00, 0x00, 0x16,
            0x0d, 0x06, 0x10, 0x28, 0x00, 0x40, 0x00, 0x01, 0x60, 0x00, 0x00, 0x02, 0x60, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x05, 0xdc, 0xc6, 0x26, 0x78, 0xd8,
            0x58, 0x7c, 0x1f, 0x58, 0x00, 0x00, 0x01, 0x62, 0x00, 0x00, 0x01, 0x30, 0xb3, 0xbf,
            0xe7, 0x02, 0xb3, 0xbf, 0xe7, 0x02, 0x01, 0xbb, 0x16, 0x7b, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0xfc, 0xdf, 0x00, 0x00, 0x18, 0x0e, 0x06, 0x10, 0x00, 0x00,
            0x40, 0x00, 0x01, 0x60, 0x00, 0x00, 0x02, 0x60, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x05, 0xdc, 0xc6, 0x26, 0x78, 0xdc, 0x5b, 0xaf, 0x13, 0x88, 0x00,
            0x00, 0x01, 0x62, 0x00, 0x00, 0x01, 0x30, 0xb3, 0xbf, 0xe7, 0x02, 0xb3, 0xbf, 0xe7,
            0x02, 0x01, 0xbb, 0x79, 0xfc, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xfc,
            0xdf, 0x00, 0x00, 0x18, 0x0d, 0x06, 0x10, 0x00, 0x00, 0x40, 0x00, 0x01, 0x60, 0x00,
            0x00, 0x02, 0x60, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x05, 0xdc,
            0xcd, 0xea, 0xaf, 0x66, 0x5b, 0xa1, 0xfc, 0x11, 0x00, 0x00, 0x01, 0x61, 0x00, 0x00,
            0x01, 0x69, 0xb3, 0xbf, 0xe7, 0x03, 0xb3, 0xbf, 0xe7, 0x03, 0x01, 0xbb, 0x79, 0x1c,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xc2, 0x95, 0xae, 0x31, 0x18, 0x0e,
            0x06, 0x10, 0x28, 0x00, 0x40, 0x00, 0x01, 0x60, 0x00, 0x00, 0x02, 0x60, 0x00, 0x00,
            0x02, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x0b, 0x20, 0x8a, 0xc7, 0x10, 0xcc, 0x5b,
            0xa6, 0xb0, 0x14, 0x00, 0x00, 0x01, 0x61, 0x00, 0x00, 0x01, 0x69, 0xb3, 0xbf, 0xe7,
            0x04, 0xb3, 0xbf, 0xe4, 0xba, 0x04, 0xaa, 0x22, 0xd9, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0xc2, 0x95, 0xae, 0x31, 0x18, 0x0e, 0x11, 0x00, 0x28, 0x00, 0x40,
            0x00, 0x01, 0x60, 0x00, 0x00, 0x02, 0x60, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x00, 0x03, 0xd8, 0xb9, 0x21, 0xdc, 0x64, 0x5b, 0xac, 0x7f, 0x22, 0x00, 0x00,
            0x01, 0x61, 0x00, 0x00, 0x01, 0x30, 0xb3, 0xbf, 0xe7, 0x04, 0xb3, 0xbf, 0xe7, 0x04,
            0x01, 0xbb, 0x1b, 0xac, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xfc, 0xdf,
            0x00, 0x00, 0x16, 0x0d, 0x06, 0x18, 0x28, 0x00, 0x40, 0x00, 0x01, 0x60, 0x00, 0x00,
            0x02, 0x60, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x05, 0xdc, 0xb9,
            0x15, 0x3d, 0x5c, 0x4e, 0xe8, 0x7a, 0x02, 0x00, 0x00, 0x01, 0x61, 0x00, 0x00, 0x01,
            0x30, 0xb3, 0xbf, 0xe7, 0x05, 0xb3, 0xbf, 0xe7, 0x05, 0x88, 0xb3, 0xd0, 0x11, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xfc, 0xdf, 0x00, 0x00, 0x16, 0x16, 0x06,
            0x10, 0x28, 0x00, 0x40, 0x00, 0x01, 0x60, 0x00, 0x00, 0x02, 0x60, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x28, 0xd4, 0x20, 0xfe, 0x7b, 0x5b, 0xab,
            0x61, 0x86, 0x00, 0x00, 0x01, 0x61, 0x00, 0x00, 0x01, 0x31, 0xb3, 0xbf, 0xe7, 0x06,
            0xb3, 0xbf, 0xe7, 0x06, 0xd3, 0xc9, 0xc3, 0x50, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0xfc, 0xdf, 0x00, 0x00, 0x13, 0x0e, 0x06, 0x10, 0x28, 0x00, 0x40, 0x00,
            0x01, 0x60, 0x00, 0x00, 0x02, 0x60, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00,
            0x00, 0x05, 0x90, 0xc6, 0x26, 0x78, 0xc3, 0x25, 0xa5, 0xad, 0xb8, 0x00, 0x00, 0x01,
            0x62, 0x00, 0x00, 0x01, 0x31, 0xb3, 0xbf, 0xe7, 0x08, 0xb3, 0xbf, 0xe7, 0x08, 0x01,
            0xbb, 0x58, 0x64, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xfc, 0xdf, 0x00,
            0x00, 0x18, 0x12, 0x06, 0x10, 0x00, 0x00, 0x40, 0x00, 0x01, 0x60, 0x00, 0x00, 0x02,
            0x60, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x05, 0xdc, 0x8f, 0xf4,
            0x39, 0x32, 0x4e, 0xe6, 0x08, 0xb1, 0x00, 0x00, 0x01, 0x61, 0x00, 0x00, 0x01, 0x30,
            0xb3, 0xbf, 0xe7, 0x08, 0xb3, 0xbf, 0xe7, 0x08, 0x01, 0xbb, 0xb2, 0x9a, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xfc, 0xdf, 0x00, 0x00, 0x17, 0x17, 0x06, 0x10,
            0x28, 0x00, 0x40, 0x00, 0x01, 0x60, 0x00, 0x00, 0x02, 0x60, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x01, 0x00, 0x00, 0x05, 0x3c, 0xc6, 0x26, 0x78, 0xb6, 0x25, 0xa4, 0xf7,
            0xb2, 0x00, 0x00, 0x01, 0x62, 0x00, 0x00, 0x01, 0x30, 0xb3, 0xbf, 0xe7, 0x09, 0xb3,
            0xbf, 0xe7, 0x09, 0x01, 0xbb, 0xdd, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0xfc, 0xdf, 0x00, 0x00, 0x18, 0x12, 0x06, 0x10, 0x00, 0x00, 0x40, 0x00, 0x01,
            0x60, 0x00, 0x00, 0x02, 0x60, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
            0x05, 0x64, 0xcd, 0xb9, 0xd8, 0x12, 0x52, 0x8e, 0x0d, 0x65, 0x00, 0x00, 0x01, 0x61,
            0x00, 0x00, 0x01, 0x30, 0xb3, 0xbf, 0xe7, 0x09, 0xb3, 0xbf, 0xe7, 0x09, 0x00, 0x50,
            0x94, 0x3c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xfc, 0xdf, 0x00, 0x00,
            0x18, 0x14, 0x06, 0x10, 0x28, 0x00, 0x40, 0x00, 0x01, 0x60, 0x00, 0x00, 0x02, 0x60,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let netflow = NetFlow::decode(&data, &mut templates).unwrap();
        println!("{:#?}", netflow);
    }
}
