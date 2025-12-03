mod ipfix;
mod netflow;

pub use ipfix::parse_ipfix_packet;
pub use netflow::parse_netflow_v9;

use super::template::{self, Field, Template, TemplateCache};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("unexpected eof")]
    UnexpectedEof,

    #[error("incompatible version {0}")]
    IncompatibleVersion(u16),

    #[error("no field in template")]
    NoFieldInTemplate,

    #[error("unknown field type {0}")]
    UnknownFieldType(u16),
}

#[derive(Debug)]
pub struct DataField<'a> {
    pub typ: u16,

    // https://datatracker.ietf.org/doc/html/rfc7011#section-6.1
    pub data: &'a [u8],
}

#[derive(Debug)]
pub struct DataRecord<'a> {
    pub fields: Vec<DataField<'a>>,
}

#[derive(Debug)]
pub struct OptionsDataRecord<'a> {
    pub scopes: Vec<DataField<'a>>,
    pub options: Vec<DataField<'a>>,
}

#[derive(Debug)]
pub enum FlowSet<'a> {
    Data {
        template: u16,
        records: Vec<DataRecord<'a>>,
    },
    OptionsData {
        template: u16,
        records: Vec<OptionsDataRecord<'a>>,
    },
}

pub struct Context<'a> {
    pub version: u16,
    pub odid: u32,
    pub templates: &'a mut TemplateCache,
}

impl Context<'_> {
    #[inline]
    pub fn add_template(&mut self, id: u16, template: Template) {
        debug!(
            message = "adding new template",
            version = self.version,
            odid = self.odid,
            template = id,
        );

        self.templates.add(self.version, self.odid, id, template);
    }
}

pub fn apply_template(buf: &[u8], mut cx: Context) -> Result<(), Error> {
    let mut pos = 0;

    while pos + 4 < buf.len() {
        let id = (buf[pos] as u16) << 8 | (buf[pos + 1] as u16);
        let field_count = ((buf[pos + 2] as u16) << 8 | (buf[pos + 3] as u16)) as usize;
        pos += 4;

        if field_count == 0 {
            return Err(Error::NoFieldInTemplate);
        }

        let mut fields = Vec::with_capacity(field_count);
        for _ in 0..field_count {
            let mut typ = (buf[pos] as u16) << 8 | (buf[pos + 1] as u16);
            let length = (buf[pos + 2] as u16) << 8 | (buf[pos + 3] as u16);
            pos += 4;

            let pen = if cx.version == 10 && typ & 0x8000 != 0 {
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

            fields.push(Field { typ, length, pen })
        }

        cx.add_template(id, Template::Basic { fields });
    }

    Ok(())
}

pub fn decode_data_records<'a>(
    buf: &'a [u8],
    template_fields: &[Field],
) -> Result<Vec<DataRecord<'a>>, Error> {
    let length = template_fields
        .iter()
        .filter(|f| f.length != 0xffff)
        .map(|field| field.length)
        .sum::<u16>() as usize;

    let mut pos = 0;
    let mut records = Vec::new();
    while pos + length < buf.len() {
        let mut fields = Vec::with_capacity(template_fields.len());
        for field in template_fields {
            let length = match field.length {
                0xffff => {
                    let a = buf[pos] as u16;
                    if a == 0xff {
                        let b = buf[pos + 1] as u16;
                        b << 8 | a
                    } else {
                        a
                    }
                }
                _ => field.length,
            };

            fields.push(DataField {
                typ: field.typ,
                data: &buf[pos..pos + length as usize],
            });
            pos += length as usize;
        }

        records.push(DataRecord { fields });
    }

    Ok(records)
}

pub fn decode_options_data_records<'a>(
    buf: &'a [u8],
    scopes_fields: &[Field],
    options_fields: &[Field],
) -> Result<Vec<OptionsDataRecord<'a>>, Error> {
    let length = scopes_fields
        .iter()
        .chain(options_fields.iter())
        .map(|field| field.length)
        .sum::<u16>() as usize;

    let mut pos = 0;
    let mut records = Vec::new();
    while pos + length < buf.len() {
        let mut scopes = Vec::with_capacity(scopes_fields.len());
        for field in scopes_fields {
            let length = match field.length {
                0xffff => {
                    let a = buf[pos] as u16;
                    if a == 0xff {
                        let b = buf[pos + 1] as u16;
                        b << 8 | a
                    } else {
                        a
                    }
                }
                _ => field.length,
            };

            scopes.push(DataField {
                typ: field.typ,
                data: &buf[pos..pos + length as usize],
            });
            pos += length as usize;
        }

        let mut options = Vec::with_capacity(options_fields.len());
        for field in options_fields {
            let length = match field.length {
                0xffff => {
                    let a = buf[pos] as u16;
                    if a == 0xff {
                        let b = buf[pos + 1] as u16;
                        b << 8 | a
                    } else {
                        a
                    }
                }
                _ => field.length,
            };

            options.push(DataField {
                typ: field.typ,
                data: &buf[pos..pos + length as usize],
            });
            pos += length as usize;
        }

        records.push(OptionsDataRecord { scopes, options });
    }

    Ok(records)
}
