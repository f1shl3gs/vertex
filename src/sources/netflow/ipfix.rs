#![allow(dead_code)]

//! Specification of the IP Flow Information Export (IPFIX) Protocol
//!
//! https://datatracker.ietf.org/doc/html/rfc7011

use std::io::Cursor;

use bytes::Buf;

use super::decode::{
    decode_data_records, decode_options_data_records, decode_template_records, DataField, Error,
};
use super::template::{Field, Template, TemplateRecord, TemplateSystem};

#[derive(Debug)]
pub struct DataRecord<'a> {
    pub fields: Vec<DataField<'a>>,
}

#[derive(Debug)]
pub struct OptionsTemplateRecord {
    pub id: u16,
    pub field_count: u16,
    pub scope_field_count: u16,
    pub options: Vec<Field>,
    pub scopes: Vec<Field>,
}

#[derive(Debug)]
pub struct OptionsDataRecord<'a> {
    pub scopes: Vec<DataField<'a>>,
    pub options: Vec<DataField<'a>>,
}

#[derive(Debug)]
pub enum FlowSet<'a> {
    Template {
        id: u16,
        length: u16,
        records: Vec<TemplateRecord>,
    },
    OptionsTemplate {
        id: u16,
        length: u16,
        records: Vec<OptionsTemplateRecord>,
    },
    Data {
        template_id: u16,
        length: u16,
        records: Vec<DataRecord<'a>>,
    },
    OptionsData {
        template_id: u16,
        length: u16,
        records: Vec<OptionsDataRecord<'a>>,
    },
}

// https://datatracker.ietf.org/doc/html/rfc7011#section-3
#[derive(Debug)]
pub struct IpFix<'a> {
    pub version: u16,
    // Total length of the IPFIX message, measured in octets, including
    // message header and set(s).
    pub length: u16,
    pub export_time: u32,
    pub sequence_number: u32,
    pub observation_domain_id: u32,

    pub flow_sets: Vec<FlowSet<'a>>,
}

impl IpFix<'_> {
    pub fn decode<'a, T: TemplateSystem>(
        data: &'a [u8],
        templates: &mut T,
    ) -> Result<IpFix<'a>, Error> {
        let mut buf = Cursor::new(data);

        let version = buf.try_get_u16()?;
        let length = buf.try_get_u16()?;
        if data.len() != length as usize {
            return Err(Error::DatagramTooShort);
        }

        let export_time = buf.try_get_u32()?;
        let sequence_number = buf.try_get_u32()?;
        let observation_domain_id = buf.try_get_u32()?;
        let flow_sets = decode_flow_sets(&mut buf, version, observation_domain_id, templates)?;

        Ok(IpFix {
            version,
            length,
            export_time,
            sequence_number,
            observation_domain_id,
            flow_sets,
        })
    }
}

fn decode_flow_sets<'a, T: TemplateSystem>(
    buf: &mut Cursor<&'a [u8]>,
    version: u16,
    observation_domain_id: u32,
    templates: &mut T,
) -> Result<Vec<FlowSet<'a>>, Error> {
    let mut flow_sets = Vec::new();
    while buf.remaining() > 0 {
        flow_sets.push(decode_flow_set(
            buf,
            version,
            observation_domain_id,
            templates,
        )?);
    }

    Ok(flow_sets)
}

fn decode_flow_set<'a, T: TemplateSystem>(
    buf: &mut Cursor<&'a [u8]>,
    version: u16,
    observation_domain_id: u32,
    templates: &mut T,
) -> Result<FlowSet<'a>, Error> {
    let id = buf.try_get_u16()?;
    // the length is the bytes length, not records array length
    let length = buf.try_get_u16()?;

    let payload = buf.get_ref();
    let start = buf.position() as usize;
    let end = start + length as usize - 4; // 4 is the length of id and length
    let mut set_buf = Cursor::new(&payload[start..end]);
    buf.set_position(end as u64);

    let set = if id == 2 {
        let records = decode_template_records(&mut set_buf, version)?;

        for template in &records {
            templates.add(
                10,
                observation_domain_id,
                template.id,
                Template::Basic {
                    fields: template.fields.clone(),
                },
            );
        }

        FlowSet::Template {
            id,
            length,
            records,
        }
    } else if id == 3 {
        let records = decode_options_template_records(&mut set_buf)?;

        for record in &records {
            templates.add(
                10,
                observation_domain_id,
                record.id,
                Template::Options {
                    scopes: record.scopes.clone(),
                    options: record.options.clone(),
                },
            );
        }

        FlowSet::OptionsTemplate {
            id,
            length,
            records,
        }
    } else if id >= 256 {
        let template =
            templates
                .get(10, observation_domain_id, id)
                .ok_or(Error::TemplateNotFound {
                    observation_domain_id,
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
                length,
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
    let mut records = Vec::new();

    while buf.remaining() >= 4 {
        let id = buf.try_get_u16()?;
        let field_count = buf.try_get_u16()?;
        let scope_field_count = buf.try_get_u16()?;

        let mut scopes = Vec::with_capacity(scope_field_count as usize);
        for _ in 0..scope_field_count {
            let typ = buf.try_get_u16()?;
            let length = buf.try_get_u16()?;
            let pen = if typ & 0x8000 != 0 {
                let pen = buf.try_get_u32()?;
                Some(pen)
            } else {
                None
            };

            scopes.push(Field { typ, length, pen });
        }

        let mut options = Vec::with_capacity(field_count as usize);
        for _ in 0..field_count {
            let typ = buf.try_get_u16()?;
            let length = buf.try_get_u16()?;
            let pen = if typ & 0x8000 != 0 {
                let pen = buf.try_get_u32()?;
                Some(pen)
            } else {
                None
            };

            options.push(Field { typ, length, pen });
        }

        records.push(OptionsTemplateRecord {
            id,
            field_count,
            scope_field_count,
            options,
            scopes,
        })
    }

    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    use super::super::template::BasicTemplateSystem;

    #[test]
    fn mikrotik() {
        let mut templates = BasicTemplateSystem::default();

        let data = [
            0, 10, 1, 56, 103, 90, 253, 77, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 1, 40, 1, 2, 0, 37, 0,
            60, 0, 1, 0, 22, 0, 4, 0, 21, 0, 4, 0, 160, 0, 8, 0, 2, 0, 8, 0, 1, 0, 8, 0, 7, 0, 2,
            0, 11, 0, 2, 0, 10, 0, 4, 0, 14, 0, 4, 0, 4, 0, 1, 0, 5, 0, 1, 0, 6, 0, 1, 0, 57, 0, 6,
            0, 80, 0, 6, 0, 81, 0, 6, 0, 56, 0, 6, 0, 8, 0, 4, 0, 12, 0, 4, 0, 15, 0, 4, 0, 9, 0,
            1, 0, 13, 0, 1, 0, 192, 0, 1, 0, 206, 0, 1, 0, 189, 0, 1, 0, 224, 0, 8, 0, 205, 0, 2,
            0, 184, 0, 4, 0, 185, 0, 4, 0, 186, 0, 2, 0, 33, 0, 1, 0, 176, 0, 1, 0, 177, 0, 1, 0,
            225, 0, 4, 0, 226, 0, 4, 0, 227, 0, 2, 0, 228, 0, 2, 1, 3, 0, 34, 0, 60, 0, 1, 0, 22,
            0, 4, 0, 21, 0, 4, 0, 160, 0, 8, 0, 2, 0, 8, 0, 1, 0, 8, 0, 7, 0, 2, 0, 11, 0, 2, 0,
            10, 0, 4, 0, 14, 0, 4, 0, 4, 0, 1, 0, 5, 0, 1, 0, 6, 0, 1, 0, 57, 0, 6, 0, 80, 0, 6, 0,
            81, 0, 6, 0, 56, 0, 6, 0, 27, 0, 16, 0, 28, 0, 16, 0, 62, 0, 16, 0, 29, 0, 1, 0, 30, 0,
            1, 0, 192, 0, 1, 0, 206, 0, 1, 0, 189, 0, 1, 0, 224, 0, 8, 0, 205, 0, 2, 0, 184, 0, 4,
            0, 185, 0, 4, 0, 186, 0, 2, 0, 33, 0, 1, 0, 178, 0, 1, 0, 179, 0, 1, 0, 31, 0, 4,
        ];
        let _packet = IpFix::decode(&data, &mut templates).unwrap();

        let data = [
            0, 10, 1, 12, 103, 90, 253, 78, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 0, 252, 4, 77, 163, 110,
            150, 77, 163, 110, 150, 0, 0, 1, 147, 109, 201, 202, 136, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0,
            0, 0, 0, 0, 7, 59, 0, 80, 217, 106, 0, 0, 0, 0, 0, 0, 0, 11, 6, 0, 24, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 220, 44, 110, 221, 85, 33, 0, 0, 0, 0, 0, 0, 192, 168, 88, 1, 192,
            168, 88, 254, 0, 0, 0, 0, 0, 0, 64, 0, 5, 0, 0, 0, 0, 0, 0, 2, 19, 0, 0, 213, 175, 134,
            96, 65, 220, 105, 54, 245, 1, 0, 0, 0, 192, 168, 88, 1, 192, 168, 88, 254, 0, 80, 217,
            106, 4, 77, 163, 110, 150, 77, 163, 110, 150, 0, 0, 1, 147, 109, 201, 202, 136, 0, 0,
            0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 3, 202, 217, 106, 0, 80, 0, 0, 0, 11, 0, 0, 0, 0,
            6, 0, 24, 220, 44, 110, 221, 85, 33, 220, 44, 110, 221, 85, 33, 0, 0, 0, 0, 0, 0, 4,
            217, 245, 249, 228, 34, 192, 168, 88, 254, 192, 168, 88, 1, 0, 0, 0, 0, 0, 0, 64, 0, 5,
            0, 0, 0, 0, 0, 0, 1, 135, 0, 0, 65, 220, 105, 54, 180, 177, 134, 96, 251, 1, 0, 0, 0,
            192, 168, 88, 254, 192, 168, 88, 1, 217, 106, 0, 80,
        ];
        let _packet = IpFix::decode(&data, &mut templates).unwrap();

        let data = [
            0, 10, 4, 236, 103, 90, 253, 81, 0, 0, 0, 2, 0, 0, 0, 0, 1, 2, 4, 220, 4, 77, 163, 121,
            74, 77, 163, 121, 74, 0, 0, 1, 147, 109, 201, 202, 136, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0,
            0, 0, 0, 0, 0, 112, 1, 187, 172, 212, 0, 0, 0, 12, 0, 0, 0, 11, 6, 0, 24, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 220, 44, 110, 221, 85, 33, 0, 0, 94, 0, 1, 203, 101, 91, 136,
            148, 125, 122, 84, 241, 0, 0, 0, 0, 0, 0, 36, 0, 5, 0, 0, 0, 0, 0, 0, 0, 72, 0, 0, 209,
            255, 207, 52, 247, 58, 47, 54, 97, 0, 0, 0, 0, 101, 91, 136, 148, 192, 168, 88, 254, 1,
            187, 172, 212, 4, 77, 163, 121, 74, 77, 163, 121, 74, 0, 0, 1, 147, 109, 201, 202, 136,
            0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 76, 172, 212, 1, 187, 0, 0, 0, 11, 0, 0,
            0, 12, 6, 0, 24, 220, 44, 110, 221, 85, 33, 220, 44, 110, 221, 85, 33, 0, 0, 0, 0, 0,
            0, 4, 217, 245, 249, 228, 34, 192, 168, 88, 254, 101, 91, 136, 148, 0, 0, 0, 0, 0, 0,
            63, 0, 5, 0, 0, 0, 0, 0, 0, 0, 76, 0, 0, 247, 58, 47, 54, 241, 255, 207, 52, 121, 2, 0,
            0, 0, 125, 122, 84, 241, 101, 91, 136, 148, 172, 212, 1, 187, 4, 77, 163, 124, 186, 77,
            163, 124, 186, 0, 0, 1, 147, 109, 201, 202, 136, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0,
            0, 0, 104, 212, 90, 0, 80, 0, 0, 0, 11, 0, 0, 0, 0, 6, 0, 16, 220, 44, 110, 221, 85,
            33, 220, 44, 110, 221, 85, 33, 0, 0, 0, 0, 0, 0, 4, 217, 245, 249, 228, 34, 192, 168,
            88, 254, 192, 168, 88, 1, 0, 0, 0, 0, 0, 0, 64, 0, 5, 0, 0, 0, 0, 0, 0, 0, 52, 0, 0,
            205, 191, 50, 145, 2, 209, 175, 47, 245, 1, 0, 0, 0, 192, 168, 88, 254, 192, 168, 88,
            1, 212, 90, 0, 80, 4, 77, 163, 124, 186, 77, 163, 124, 186, 0, 0, 1, 147, 109, 201,
            202, 136, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 156, 0, 80, 212, 90, 0, 0, 0, 0,
            0, 0, 0, 11, 6, 0, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 220, 44, 110, 221, 85, 33,
            0, 0, 0, 0, 0, 0, 192, 168, 88, 1, 192, 168, 88, 254, 0, 0, 0, 0, 0, 0, 64, 0, 5, 0, 0,
            0, 0, 0, 0, 0, 52, 0, 0, 2, 209, 175, 47, 206, 191, 50, 145, 251, 1, 0, 0, 0, 192, 168,
            88, 1, 192, 168, 88, 254, 0, 80, 212, 90, 4, 77, 163, 124, 186, 77, 163, 124, 186, 0,
            0, 1, 147, 109, 201, 202, 136, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 104, 212,
            96, 0, 80, 0, 0, 0, 11, 0, 0, 0, 0, 6, 0, 16, 220, 44, 110, 221, 85, 33, 220, 44, 110,
            221, 85, 33, 0, 0, 0, 0, 0, 0, 4, 217, 245, 249, 228, 34, 192, 168, 88, 254, 192, 168,
            88, 1, 0, 0, 0, 0, 0, 0, 64, 0, 5, 0, 0, 0, 0, 0, 0, 0, 52, 0, 0, 152, 64, 107, 156,
            171, 188, 90, 149, 245, 1, 0, 0, 0, 192, 168, 88, 254, 192, 168, 88, 1, 212, 96, 0, 80,
            4, 77, 163, 124, 186, 77, 163, 124, 186, 0, 0, 1, 147, 109, 201, 202, 136, 0, 0, 0, 0,
            0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 156, 0, 80, 212, 96, 0, 0, 0, 0, 0, 0, 0, 11, 6, 0,
            16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 220, 44, 110, 221, 85, 33, 0, 0, 0, 0, 0, 0,
            192, 168, 88, 1, 192, 168, 88, 254, 0, 0, 0, 0, 0, 0, 64, 0, 5, 0, 0, 0, 0, 0, 0, 0,
            52, 0, 0, 171, 188, 90, 149, 153, 64, 107, 156, 251, 1, 0, 0, 0, 192, 168, 88, 1, 192,
            168, 88, 254, 0, 80, 212, 96, 4, 77, 163, 124, 196, 77, 163, 124, 196, 0, 0, 1, 147,
            109, 201, 202, 136, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 104, 0, 80, 212, 84,
            0, 0, 0, 0, 0, 0, 0, 11, 6, 0, 17, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 220, 44, 110,
            221, 85, 33, 0, 0, 0, 0, 0, 0, 192, 168, 88, 1, 192, 168, 88, 254, 0, 0, 0, 0, 0, 0,
            64, 0, 5, 0, 0, 0, 0, 0, 0, 0, 52, 0, 0, 38, 239, 100, 130, 101, 243, 67, 215, 251, 1,
            0, 0, 0, 192, 168, 88, 1, 192, 168, 88, 254, 0, 80, 212, 84, 4, 77, 163, 124, 196, 77,
            163, 124, 196, 0, 0, 1, 147, 109, 201, 202, 136, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0,
            0, 0, 52, 212, 84, 0, 80, 0, 0, 0, 11, 0, 0, 0, 0, 6, 0, 17, 220, 44, 110, 221, 85, 33,
            220, 44, 110, 221, 85, 33, 0, 0, 0, 0, 0, 0, 4, 217, 245, 249, 228, 34, 192, 168, 88,
            254, 192, 168, 88, 1, 0, 0, 0, 0, 0, 0, 64, 0, 5, 0, 0, 0, 0, 0, 0, 0, 52, 0, 0, 101,
            243, 67, 215, 39, 239, 100, 130, 245, 1, 0, 0, 0, 192, 168, 88, 254, 192, 168, 88, 1,
            212, 84, 0, 80, 4, 77, 163, 126, 4, 77, 163, 126, 4, 0, 0, 1, 147, 109, 201, 202, 136,
            0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 1, 28, 171, 140, 1, 187, 0, 0, 0, 11, 0, 0,
            0, 12, 6, 0, 24, 220, 44, 110, 221, 85, 33, 220, 44, 110, 221, 85, 33, 0, 0, 0, 0, 0,
            0, 4, 217, 245, 249, 228, 34, 192, 168, 88, 254, 111, 77, 198, 225, 0, 0, 0, 0, 0, 0,
            63, 0, 5, 0, 0, 0, 0, 0, 0, 0, 169, 0, 0, 139, 29, 113, 151, 122, 113, 112, 5, 201, 1,
            0, 0, 0, 125, 122, 84, 241, 111, 77, 198, 225, 171, 140, 1, 187, 4, 77, 163, 126, 24,
            77, 163, 126, 24, 0, 0, 1, 147, 109, 201, 202, 136, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0,
            0, 0, 2, 13, 1, 187, 171, 140, 0, 0, 0, 12, 0, 0, 0, 11, 6, 0, 16, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 220, 44, 110, 221, 85, 33, 0, 0, 94, 0, 1, 203, 111, 77, 198, 225, 125,
            122, 84, 241, 0, 0, 0, 0, 0, 0, 55, 0, 5, 0, 0, 0, 0, 0, 0, 0, 40, 0, 0, 122, 113, 112,
            5, 12, 30, 113, 151, 83, 0, 0, 0, 0, 111, 77, 198, 225, 192, 168, 88, 254, 1, 187, 171,
            140,
        ];
        let _packet = IpFix::decode(&data, &mut templates).unwrap();

        let data = [
            0, 10, 5, 104, 103, 90, 253, 84, 0, 0, 0, 12, 0, 0, 0, 0, 1, 2, 5, 88, 4, 77, 163, 132,
            128, 77, 163, 132, 128, 0, 0, 1, 147, 109, 201, 202, 136, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0,
            0, 0, 0, 0, 0, 112, 1, 187, 160, 216, 0, 0, 0, 12, 0, 0, 0, 11, 6, 0, 24, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 220, 44, 110, 221, 85, 33, 0, 0, 94, 0, 1, 203, 101, 91, 133,
            30, 125, 122, 84, 241, 0, 0, 0, 0, 0, 0, 42, 0, 5, 0, 0, 0, 0, 0, 0, 0, 72, 0, 0, 114,
            86, 238, 89, 188, 75, 228, 253, 75, 0, 0, 0, 0, 101, 91, 133, 30, 192, 168, 88, 254, 1,
            187, 160, 216, 4, 77, 163, 132, 128, 77, 163, 132, 128, 0, 0, 1, 147, 109, 201, 202,
            136, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 76, 160, 216, 1, 187, 0, 0, 0, 11, 0,
            0, 0, 12, 6, 0, 24, 220, 44, 110, 221, 85, 33, 220, 44, 110, 221, 85, 33, 0, 0, 0, 0,
            0, 0, 4, 217, 245, 249, 228, 34, 192, 168, 88, 254, 101, 91, 133, 30, 0, 0, 0, 0, 0, 0,
            63, 0, 5, 0, 0, 0, 0, 0, 0, 0, 76, 0, 0, 188, 75, 228, 253, 146, 86, 238, 89, 121, 2,
            0, 0, 0, 125, 122, 84, 241, 101, 91, 133, 30, 160, 216, 1, 187, 4, 77, 163, 136, 64,
            77, 163, 136, 64, 0, 0, 1, 147, 109, 201, 202, 136, 0, 0, 0, 0, 0, 0, 0, 25, 0, 0, 0,
            0, 0, 0, 52, 85, 158, 58, 1, 187, 0, 0, 0, 11, 0, 0, 0, 12, 6, 0, 2, 220, 44, 110, 221,
            85, 33, 220, 44, 110, 221, 85, 33, 0, 0, 0, 0, 0, 0, 4, 217, 245, 249, 228, 34, 192,
            168, 88, 254, 175, 4, 62, 126, 0, 0, 0, 0, 0, 0, 63, 0, 5, 0, 0, 0, 0, 0, 0, 0, 60, 0,
            0, 175, 128, 97, 110, 0, 0, 0, 0, 240, 250, 0, 0, 0, 125, 122, 84, 241, 175, 4, 62,
            126, 158, 58, 1, 187, 4, 77, 163, 136, 84, 77, 163, 136, 84, 0, 0, 1, 147, 109, 201,
            202, 136, 0, 0, 0, 0, 0, 0, 0, 23, 0, 0, 0, 0, 0, 0, 28, 183, 1, 187, 158, 58, 0, 0, 0,
            12, 0, 0, 0, 11, 6, 0, 18, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 220, 44, 110, 221, 85,
            33, 0, 0, 0, 0, 0, 0, 175, 4, 62, 126, 125, 122, 84, 241, 0, 0, 0, 0, 0, 0, 52, 0, 5,
            0, 0, 0, 0, 0, 0, 0, 52, 0, 0, 33, 154, 34, 47, 176, 128, 97, 110, 100, 165, 0, 0, 0,
            175, 4, 62, 126, 192, 168, 88, 254, 1, 187, 158, 58, 4, 77, 163, 139, 246, 77, 163,
            139, 246, 0, 0, 1, 147, 109, 201, 202, 136, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0,
            0, 131, 104, 77, 4, 162, 0, 0, 0, 11, 0, 0, 0, 12, 17, 0, 0, 220, 44, 110, 221, 85, 33,
            220, 44, 110, 221, 85, 33, 0, 0, 0, 0, 0, 0, 4, 217, 245, 249, 228, 34, 192, 168, 88,
            254, 182, 46, 1, 75, 0, 0, 0, 0, 0, 0, 63, 0, 5, 0, 0, 0, 0, 0, 0, 0, 131, 0, 111, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 125, 122, 84, 241, 182, 46, 1, 75, 104, 77, 4, 162,
            4, 77, 163, 139, 246, 77, 163, 139, 246, 0, 0, 1, 147, 109, 201, 202, 136, 0, 0, 0, 0,
            0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 131, 104, 77, 88, 103, 0, 0, 0, 11, 0, 0, 0, 12, 17,
            0, 0, 220, 44, 110, 221, 85, 33, 220, 44, 110, 221, 85, 33, 0, 0, 0, 0, 0, 0, 4, 217,
            245, 249, 228, 34, 192, 168, 88, 254, 185, 75, 226, 172, 0, 0, 0, 0, 0, 0, 63, 0, 5, 0,
            0, 0, 0, 0, 0, 0, 131, 0, 111, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 125, 122, 84,
            241, 185, 75, 226, 172, 104, 77, 88, 103, 4, 77, 163, 140, 0, 77, 163, 140, 0, 0, 0, 1,
            147, 109, 201, 202, 136, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 131, 104, 77, 81,
            74, 0, 0, 0, 11, 0, 0, 0, 12, 17, 0, 0, 220, 44, 110, 221, 85, 33, 220, 44, 110, 221,
            85, 33, 0, 0, 0, 0, 0, 0, 4, 217, 245, 249, 228, 34, 192, 168, 88, 254, 62, 201, 255,
            83, 0, 0, 0, 0, 0, 0, 63, 0, 5, 0, 0, 0, 0, 0, 0, 0, 131, 0, 111, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 125, 122, 84, 241, 62, 201, 255, 83, 104, 77, 81, 74, 4, 77, 163,
            140, 0, 77, 163, 140, 0, 0, 0, 1, 147, 109, 201, 202, 136, 0, 0, 0, 0, 0, 0, 0, 1, 0,
            0, 0, 0, 0, 0, 0, 131, 104, 77, 176, 8, 0, 0, 0, 11, 0, 0, 0, 12, 17, 0, 0, 220, 44,
            110, 221, 85, 33, 220, 44, 110, 221, 85, 33, 0, 0, 0, 0, 0, 0, 4, 217, 245, 249, 228,
            34, 192, 168, 88, 254, 69, 50, 95, 167, 0, 0, 0, 0, 0, 0, 63, 0, 5, 0, 0, 0, 0, 0, 0,
            0, 131, 0, 111, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 125, 122, 84, 241, 69, 50, 95,
            167, 104, 77, 176, 8, 4, 77, 163, 140, 0, 77, 163, 140, 0, 0, 0, 1, 147, 109, 201, 202,
            136, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 131, 104, 77, 128, 170, 0, 0, 0, 11,
            0, 0, 0, 12, 17, 0, 0, 220, 44, 110, 221, 85, 33, 220, 44, 110, 221, 85, 33, 0, 0, 0,
            0, 0, 0, 4, 217, 245, 249, 228, 34, 192, 168, 88, 254, 69, 50, 95, 167, 0, 0, 0, 0, 0,
            0, 63, 0, 5, 0, 0, 0, 0, 0, 0, 0, 131, 0, 111, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            125, 122, 84, 241, 69, 50, 95, 167, 104, 77, 128, 170, 4, 77, 163, 140, 0, 77, 163,
            140, 0, 0, 0, 1, 147, 109, 201, 202, 136, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0,
            131, 104, 77, 146, 151, 0, 0, 0, 11, 0, 0, 0, 12, 17, 0, 0, 220, 44, 110, 221, 85, 33,
            220, 44, 110, 221, 85, 33, 0, 0, 0, 0, 0, 0, 4, 217, 245, 249, 228, 34, 192, 168, 88,
            254, 83, 8, 180, 132, 0, 0, 0, 0, 0, 0, 63, 0, 5, 0, 0, 0, 0, 0, 0, 0, 131, 0, 111, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 125, 122, 84, 241, 83, 8, 180, 132, 104, 77, 146,
            151, 4, 77, 163, 140, 20, 77, 163, 140, 20, 0, 0, 1, 147, 109, 201, 202, 136, 0, 0, 0,
            0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 159, 4, 162, 104, 77, 0, 0, 0, 12, 0, 0, 0, 11, 17,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 220, 44, 110, 221, 85, 33, 0, 0, 94, 0, 1,
            203, 182, 46, 1, 75, 125, 122, 84, 241, 0, 0, 0, 0, 0, 0, 56, 0, 5, 0, 0, 0, 0, 0, 0,
            0, 159, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 182, 46, 1, 75, 192, 168, 88, 254,
            4, 162, 104, 77,
        ];
        let _packet = IpFix::decode(&data, &mut templates).unwrap();

        let data = [
            0, 10, 5, 104, 103, 90, 253, 85, 0, 0, 0, 23, 0, 0, 0, 0, 1, 2, 5, 88, 4, 77, 163, 140,
            100, 77, 163, 140, 100, 0, 0, 1, 147, 109, 201, 202, 136, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0,
            0, 0, 0, 0, 7, 32, 67, 55, 104, 77, 0, 0, 0, 12, 0, 0, 0, 11, 17, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 220, 44, 110, 221, 85, 33, 0, 0, 94, 0, 1, 203, 222, 187, 254, 73,
            125, 122, 84, 241, 0, 0, 0, 0, 0, 0, 51, 0, 5, 0, 0, 0, 0, 0, 0, 2, 96, 2, 76, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 222, 187, 254, 73, 192, 168, 88, 254, 67, 55, 104, 77, 4,
            77, 163, 140, 180, 77, 163, 140, 180, 0, 0, 1, 147, 109, 201, 202, 136, 0, 0, 0, 0, 0,
            0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 159, 176, 8, 104, 77, 0, 0, 0, 12, 0, 0, 0, 11, 17, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 220, 44, 110, 221, 85, 33, 0, 0, 94, 0, 1, 203, 69,
            50, 95, 167, 125, 122, 84, 241, 0, 0, 0, 0, 0, 0, 49, 0, 5, 0, 0, 0, 0, 0, 0, 0, 159,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 69, 50, 95, 167, 192, 168, 88, 254, 176,
            8, 104, 77, 4, 77, 163, 141, 64, 77, 163, 141, 64, 0, 0, 1, 147, 109, 201, 202, 136, 0,
            0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 71, 88, 103, 104, 77, 0, 0, 0, 12, 0, 0, 0,
            11, 17, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 220, 44, 110, 221, 85, 33, 0, 0, 94,
            0, 1, 203, 185, 75, 226, 172, 125, 122, 84, 241, 0, 0, 0, 0, 0, 0, 38, 0, 5, 0, 0, 0,
            0, 0, 0, 1, 71, 1, 51, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 185, 75, 226, 172, 192,
            168, 88, 254, 88, 103, 104, 77, 4, 77, 163, 141, 64, 77, 163, 141, 64, 0, 0, 1, 147,
            109, 201, 202, 136, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 131, 104, 77, 47, 62,
            0, 0, 0, 11, 0, 0, 0, 12, 17, 0, 0, 220, 44, 110, 221, 85, 33, 220, 44, 110, 221, 85,
            33, 0, 0, 0, 0, 0, 0, 4, 217, 245, 249, 228, 34, 192, 168, 88, 254, 176, 29, 78, 198,
            0, 0, 0, 0, 0, 0, 63, 0, 5, 0, 0, 0, 0, 0, 0, 0, 131, 0, 111, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 125, 122, 84, 241, 176, 29, 78, 198, 104, 77, 47, 62, 4, 77, 163, 141,
            154, 77, 163, 141, 154, 0, 0, 1, 147, 109, 201, 202, 136, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0,
            0, 0, 0, 0, 1, 71, 81, 74, 104, 77, 0, 0, 0, 12, 0, 0, 0, 11, 17, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 220, 44, 110, 221, 85, 33, 0, 0, 94, 0, 1, 203, 62, 201, 255, 83,
            125, 122, 84, 241, 0, 0, 0, 0, 0, 0, 38, 0, 5, 0, 0, 0, 0, 0, 0, 1, 71, 1, 51, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 62, 201, 255, 83, 192, 168, 88, 254, 81, 74, 104, 77, 4,
            77, 163, 141, 154, 77, 163, 141, 154, 0, 0, 1, 147, 109, 201, 202, 136, 0, 0, 0, 0, 0,
            0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 131, 104, 77, 187, 152, 0, 0, 0, 11, 0, 0, 0, 12, 17, 0,
            0, 220, 44, 110, 221, 85, 33, 220, 44, 110, 221, 85, 33, 0, 0, 0, 0, 0, 0, 4, 217, 245,
            249, 228, 34, 192, 168, 88, 254, 178, 233, 44, 42, 0, 0, 0, 0, 0, 0, 63, 0, 5, 0, 0, 0,
            0, 0, 0, 0, 131, 0, 111, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 125, 122, 84, 241, 178,
            233, 44, 42, 104, 77, 187, 152, 4, 77, 163, 142, 138, 77, 163, 142, 138, 0, 0, 1, 147,
            109, 201, 202, 136, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 71, 47, 62, 104, 77,
            0, 0, 0, 12, 0, 0, 0, 11, 17, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 220, 44, 110,
            221, 85, 33, 0, 0, 94, 0, 1, 203, 176, 29, 78, 198, 125, 122, 84, 241, 0, 0, 0, 0, 0,
            0, 43, 0, 5, 0, 0, 0, 0, 0, 0, 1, 71, 1, 51, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            176, 29, 78, 198, 192, 168, 88, 254, 47, 62, 104, 77, 4, 77, 163, 142, 138, 77, 163,
            142, 138, 0, 0, 1, 147, 109, 201, 202, 136, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0,
            0, 131, 104, 77, 26, 225, 0, 0, 0, 11, 0, 0, 0, 12, 17, 0, 0, 220, 44, 110, 221, 85,
            33, 220, 44, 110, 221, 85, 33, 0, 0, 0, 0, 0, 0, 4, 217, 245, 249, 228, 34, 192, 168,
            88, 254, 5, 167, 214, 126, 0, 0, 0, 0, 0, 0, 63, 0, 5, 0, 0, 0, 0, 0, 0, 0, 131, 0,
            111, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 125, 122, 84, 241, 5, 167, 214, 126, 104,
            77, 26, 225, 4, 77, 163, 143, 122, 77, 163, 143, 122, 0, 0, 1, 147, 109, 201, 202, 136,
            0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 71, 26, 225, 104, 77, 0, 0, 0, 12, 0, 0,
            0, 11, 17, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 220, 44, 110, 221, 85, 33, 0, 0,
            94, 0, 1, 203, 5, 167, 214, 126, 125, 122, 84, 241, 0, 0, 0, 0, 0, 0, 115, 0, 5, 0, 0,
            0, 0, 0, 0, 1, 71, 1, 51, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 167, 214, 126, 192,
            168, 88, 254, 26, 225, 104, 77, 4, 77, 163, 143, 122, 77, 163, 143, 122, 0, 0, 1, 147,
            109, 201, 202, 136, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 131, 104, 77, 216,
            192, 0, 0, 0, 11, 0, 0, 0, 12, 17, 0, 0, 220, 44, 110, 221, 85, 33, 220, 44, 110, 221,
            85, 33, 0, 0, 0, 0, 0, 0, 4, 217, 245, 249, 228, 34, 192, 168, 88, 254, 176, 59, 138,
            101, 0, 0, 0, 0, 0, 0, 63, 0, 5, 0, 0, 0, 0, 0, 0, 0, 131, 0, 111, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 125, 122, 84, 241, 176, 59, 138, 101, 104, 77, 216, 192, 4, 77, 163,
            143, 122, 77, 163, 143, 122, 0, 0, 1, 147, 109, 201, 202, 136, 0, 0, 0, 0, 0, 0, 0, 1,
            0, 0, 0, 0, 0, 0, 0, 131, 104, 77, 176, 13, 0, 0, 0, 11, 0, 0, 0, 12, 17, 0, 0, 220,
            44, 110, 221, 85, 33, 220, 44, 110, 221, 85, 33, 0, 0, 0, 0, 0, 0, 4, 217, 245, 249,
            228, 34, 192, 168, 88, 254, 137, 74, 95, 127, 0, 0, 0, 0, 0, 0, 63, 0, 5, 0, 0, 0, 0,
            0, 0, 0, 131, 0, 111, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 125, 122, 84, 241, 137,
            74, 95, 127, 104, 77, 176, 13,
        ];
        let _packet = IpFix::decode(&data, &mut templates).unwrap();

        let data = [
            0, 10, 2, 4, 103, 90, 253, 87, 0, 0, 0, 34, 0, 0, 0, 0, 1, 2, 1, 244, 4, 77, 163, 144,
            196, 77, 163, 144, 196, 0, 0, 1, 147, 109, 201, 202, 136, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0,
            0, 0, 0, 0, 0, 159, 216, 192, 104, 77, 0, 0, 0, 12, 0, 0, 0, 11, 17, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 220, 44, 110, 221, 85, 33, 0, 0, 94, 0, 1, 203, 176, 59, 138,
            101, 125, 122, 84, 241, 0, 0, 0, 0, 0, 0, 48, 0, 5, 0, 0, 0, 0, 0, 0, 0, 159, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 176, 59, 138, 101, 192, 168, 88, 254, 216, 192,
            104, 77, 4, 77, 163, 149, 36, 77, 163, 149, 36, 0, 0, 1, 147, 109, 201, 202, 136, 0, 0,
            0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 60, 134, 156, 3, 85, 0, 0, 0, 11, 0, 0, 0, 12,
            6, 0, 2, 220, 44, 110, 221, 85, 33, 220, 44, 110, 221, 85, 33, 0, 0, 0, 0, 0, 0, 4,
            217, 245, 249, 228, 34, 192, 168, 88, 254, 8, 8, 8, 8, 0, 0, 0, 0, 0, 0, 63, 0, 5, 0,
            0, 0, 0, 0, 0, 0, 60, 0, 0, 88, 180, 63, 244, 0, 0, 0, 0, 240, 250, 0, 0, 0, 125, 122,
            84, 241, 8, 8, 8, 8, 134, 156, 3, 85, 4, 77, 163, 149, 36, 77, 163, 149, 36, 0, 0, 1,
            147, 109, 201, 202, 136, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 60, 237, 164, 1,
            187, 0, 0, 0, 11, 0, 0, 0, 12, 6, 0, 2, 220, 44, 110, 221, 85, 33, 220, 44, 110, 221,
            85, 33, 0, 0, 0, 0, 0, 0, 4, 217, 245, 249, 228, 34, 192, 168, 88, 254, 8, 8, 4, 4, 0,
            0, 0, 0, 0, 0, 63, 0, 5, 0, 0, 0, 0, 0, 0, 0, 60, 0, 0, 15, 3, 30, 238, 0, 0, 0, 0,
            240, 250, 0, 0, 0, 125, 122, 84, 241, 8, 8, 4, 4, 237, 164, 1, 187, 4, 77, 163, 150,
            20, 77, 163, 150, 20, 0, 0, 1, 147, 109, 201, 202, 136, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0,
            0, 0, 0, 0, 1, 215, 1, 187, 199, 146, 0, 0, 0, 12, 0, 0, 0, 11, 6, 0, 16, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 220, 44, 110, 221, 85, 33, 0, 0, 94, 0, 1, 203, 162, 159, 36,
            1, 125, 122, 84, 241, 0, 0, 0, 0, 0, 0, 53, 0, 5, 0, 0, 0, 0, 0, 0, 0, 52, 0, 0, 246,
            63, 156, 172, 84, 68, 248, 57, 20, 0, 0, 0, 0, 162, 159, 36, 1, 192, 168, 88, 254, 1,
            187, 199, 146,
        ];
        let _packet = IpFix::decode(&data, &mut templates).unwrap();
    }

    #[test]
    fn ipfixcol2() {
        let mut templates = BasicTemplateSystem::default();

        // the data contains multiple message
        let data = std::fs::read("tests/netflow/flows.ipfix").unwrap();

        // split and decode
        let mut buf = Cursor::new(data.as_slice());
        while buf.remaining() > 0 {
            let start = buf.position();
            let version = buf.try_get_u16().unwrap();
            assert_eq!(version, 10);

            let length = buf.try_get_u16().unwrap();
            let end = start + length as u64;
            buf.set_position(end);

            let buf = &buf.get_ref()[start as usize..end as usize];

            let _packet = IpFix::decode(buf, &mut templates).unwrap();
        }
    }
}
