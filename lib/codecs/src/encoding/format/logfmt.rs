use std::fmt::Write;

use bytes::BytesMut;
use event::Event;
use tokio_util::codec::Encoder;

use super::SerializeError;

/// Serializer that converts an `Event` to bytes using the logfmt format
#[derive(Clone, Debug)]
pub struct LogfmtSerializer;

impl LogfmtSerializer {
    /// Creates a new `LogfmtSerializer`
    pub const fn new() -> Self {
        Self
    }
}

impl Encoder<Event> for LogfmtSerializer {
    type Error = SerializeError;

    fn encode(&mut self, event: Event, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let log = event.into_log();
        if let Some(fields) = log.all_fields() {
            for (k, v) in fields {
                encode_string(dst, k.as_str())?;
                dst.write_char('=')?;
                encode_string(dst, v.to_string_lossy().as_ref())?;
                dst.write_char(' ')?;
            }
        }

        // strip the final delimiter
        dst.truncate(dst.len() - 1);

        Ok(())
    }
}

fn encode_string(output: &mut BytesMut, str: &str) -> Result<(), SerializeError> {
    let needs_quoting = str
        .chars()
        .any(|c| c.is_whitespace() || c == '"' || c == '=');

    if !needs_quoting {
        output.write_str(str)?;
        return Ok(());
    }

    output.write_char('"').unwrap();
    for c in str.chars() {
        match c {
            '\\' => output.write_char('\\')?,
            '"' => output.write_char('\"')?,
            '\n' => output.write_char('\n')?,
            _ => output.write_char(c)?,
        }
    }
    output.write_char('"')?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use value::value;

    use super::*;

    #[test]
    fn encode_log() {
        let mut s = LogfmtSerializer::new();
        let log = value!({
            "foo": "bar",
            "m1": {},
            "arr": [
                1
            ],
            "m2": {
                "i64": 1,
                "f64": 1.1,
                "bool": true,
                "map": {
                    "foo": "bar"
                }
            }
        });

        let mut buf = BytesMut::new();
        s.encode(log.into(), &mut buf).unwrap();
    }
}
