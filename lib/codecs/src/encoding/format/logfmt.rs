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
    use event::fields;
    use event::log::Value;

    use super::*;

    #[test]
    fn encode_log() {
        let mut s = LogfmtSerializer::new();
        let log = fields!(
            "foo" => "bar",
            "m1" => fields!(),
            "arr" => vec![
                Value::from(1)
            ],
            "m2" => fields!(
                "i64" => 1,
                "f64" => 1.1,
                "bool" => true,
                "map" => fields!(
                    "foo" => "bar"
                )
            )
        );

        let mut buf = BytesMut::new();
        s.encode(log.into(), &mut buf).unwrap();
    }

    /*
    fn flatten(input: &BTreeMap<String, Value>, separator: char) -> BTreeMap<String, String> {
        let mut map = BTreeMap::new();

        for (k, v) in input {
            match v {
                // TODO: array
                Value::Object(m) => {
                    for (nk, nv) in flatten(m, separator) {
                        map.insert(format!("{}{}{}", k, separator, nk), nv);
                    }
                }
                _ => {
                    map.insert(k.to_string(), v.to_string_lossy());
                }
            }
        }

        map
    }

    #[test]
    fn flatten_nest_object() {
        let input = fields!(
            "foo" => "bar",
            "m1" => fields!(),
            "m2" => fields!(
                "i64" => 1,
                "f64" => 1.1,
                "bool" => true,
                "map" => fields!(
                    "foo" => "bar"
                )
            )
        );

        let got = flatten(&input, '.');

        assert_eq!(got.get("foo").unwrap(), "bar");
        assert_eq!(got.get("m2.i64").unwrap(), "1");
        assert_eq!(got.get("m2.f64").unwrap(), "1.1");
        assert_eq!(got.get("m2.bool").unwrap(), "true");
        assert_eq!(got.get("m2.map.foo").unwrap(), "bar");
    }

    #[test]
    fn serialize() {
        let event = Event::from(fields!(
            "foo" => "bar",
            "map" => fields!(
                "a" => 1,
                "b" => 2.1,
            )
        ));
        let mut serializer = LogfmtSerializer::new();
        let mut buf = BytesMut::new();
        serializer.encode(event, &mut buf).unwrap();

        assert_eq!(buf.freeze(), "foo=bar map.a=1 map.b=2.1")
    }*/
}
