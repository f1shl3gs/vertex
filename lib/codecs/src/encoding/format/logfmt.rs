use std::collections::BTreeMap;
use std::fmt::Write;

use bytes::BytesMut;
use event::log::Value;
use event::Event;
use tokio_util::codec::Encoder;

#[derive(Clone)]
pub struct LogfmtSerializer;

impl LogfmtSerializer {
    pub const fn new() -> Self {
        Self
    }
}

impl Encoder<Event> for LogfmtSerializer {
    type Error = crate::Error;

    fn encode(&mut self, item: Event, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let log = item.as_log();
        let map = flatten(&log.fields, '.');
        let last = map.len() - 1;

        for (index, (k, v)) in map.iter().enumerate() {
            dst.write_str(k)?;
            dst.write_char('=')?;
            dst.write_str(v)?;

            if index != last {
                dst.write_char(' ')?;
            }
        }

        Ok(())
    }
}

fn flatten(input: &BTreeMap<String, Value>, separator: char) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();

    for (k, v) in input {
        match v {
            // TODO: array
            Value::Map(m) => {
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

#[cfg(test)]
mod tests {
    use super::*;
    use event::fields;

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
    }
}
