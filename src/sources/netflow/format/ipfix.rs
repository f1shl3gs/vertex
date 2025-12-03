//! https://www.rfc-editor.org/rfc/rfc5102.html#section-5.4.15

use super::template::{Field, Template, TemplateCache};
use super::{Context, Error, FlowSet};
use super::{apply_template, decode_data_records, decode_options_data_records};

fn apply_ipfix_options_template(buf: &[u8], mut cx: Context) -> Result<(), Error> {
    let mut pos = 0;

    while pos + 6 < buf.len() {
        let id = (buf[pos] as u16) << 8 | (buf[pos + 1] as u16);
        let field_count = (buf[pos + 2] as u16) << 8 | (buf[pos + 3] as u16);
        let scope_count = (buf[pos + 4] as u16) << 8 | (buf[pos + 5] as u16);
        pos += 6;

        let mut scopes = Vec::with_capacity(scope_count as usize);
        for _ in 0..scope_count {
            let mut typ = (buf[pos] as u16) << 8 | (buf[pos + 1] as u16);
            let length = (buf[pos + 2] as u16) << 8 | (buf[pos + 3] as u16);
            pos += 4;

            let pen = if typ & 0x8000 != 0 {
                if pos + 4 > buf.len() {
                    return Err(Error::UnexpectedEof);
                }

                typ ^= 0x8000;
                let pen = unsafe { std::ptr::read_unaligned(buf.as_ptr().add(pos) as *const u32) }
                    .to_be();
                pos += 4;

                Some(pen)
            } else {
                None
            };

            scopes.push(Field { typ, length, pen })
        }

        let mut options = Vec::with_capacity((field_count - scope_count) as usize);
        for _ in 0..field_count - scope_count {
            let mut typ = (buf[pos] as u16) << 8 | (buf[pos + 1] as u16);
            let length = (buf[pos + 2] as u16) << 8 | (buf[pos + 3] as u16);
            pos += 4;

            let pen = if typ & 0x8000 != 0 {
                if pos + 4 > buf.len() {
                    return Err(Error::UnexpectedEof);
                }

                typ ^= 0x8000;
                let pen = unsafe { std::ptr::read_unaligned(buf.as_ptr().add(pos) as *const u32) }
                    .to_be();
                pos += 4;

                Some(pen)
            } else {
                None
            };

            options.push(Field { typ, length, pen })
        }

        cx.add_template(id, Template::Options { scopes, options });
    }

    Ok(())
}

pub struct IpfixHeader {
    pub export_time: u32,
    pub sequence_number: u32,
    pub odid: u32,
}

pub fn parse_ipfix_packet<'a>(
    buf: &'a [u8],
    templates: &mut TemplateCache,
) -> Result<(IpfixHeader, Vec<FlowSet<'a>>), Error> {
    if buf.len() < 16 {
        return Err(Error::UnexpectedEof);
    }

    let length = (buf[2] as u16) << 8 | (buf[3] as u16);
    if buf.len() != length as usize {
        return Err(Error::UnexpectedEof);
    }

    let export_time =
        (buf[4] as u32) << 24 | (buf[5] as u32) << 16 | (buf[6] as u32) << 8 | (buf[7] as u32);
    let sequence_number =
        (buf[8] as u32) << 24 | (buf[9] as u32) << 16 | (buf[10] as u32) << 8 | (buf[11] as u32);
    let odid =
        (buf[12] as u32) << 24 | (buf[13] as u32) << 16 | (buf[14] as u32) << 8 | (buf[15] as u32);

    let mut pos = 16;
    let mut flow_sets = Vec::new();
    while pos + 4 < buf.len() {
        let id = (buf[pos] as u16) << 8 | (buf[pos + 1] as u16);
        let length = ((buf[pos + 2] as u16) << 8 | (buf[pos + 3] as u16)) as usize;
        if pos + length > buf.len() {
            return Err(Error::UnexpectedEof);
        }

        let data = &buf[pos + 4..pos + length];
        let cx = Context {
            version: 10,
            odid,
            templates,
        };
        match id {
            2 => apply_template(data, cx)?,
            3 => apply_ipfix_options_template(data, cx)?,
            id if id >= 256 => {
                let Some(template) = templates.get(10, odid, id) else {
                    warn!(
                        message = "unknown template",
                        version = 10,
                        odid,
                        template = id,
                        internal_log_rate_limit = true
                    );

                    pos += length;
                    continue;
                };

                let set = match template {
                    Template::Basic { fields } => FlowSet::Data {
                        template: id,
                        records: decode_data_records(data, fields)?,
                    },
                    Template::Options { scopes, options } => FlowSet::OptionsData {
                        template: id,
                        records: decode_options_data_records(data, scopes, options)?,
                    },
                };

                flow_sets.push(set);
            }
            _ => {
                warn!(
                    message = "invalid flow set id",
                    version = 10,
                    odid,
                    id,
                    internal_log_rate_limit = 30,
                );
            }
        }

        pos += length;
    }

    Ok((
        IpfixHeader {
            export_time,
            sequence_number,
            odid,
        },
        flow_sets,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mikrotik() {
        // template only
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

        let mut templates = TemplateCache::default();
        let (_header, flow_sets) = parse_ipfix_packet(data.as_slice(), &mut templates).unwrap();
        assert!(flow_sets.is_empty());

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

        parse_ipfix_packet(data.as_slice(), &mut templates).unwrap();
    }
}
