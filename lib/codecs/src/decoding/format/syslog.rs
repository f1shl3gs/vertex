use std::collections::BTreeMap;
use std::fmt::{Display, Formatter, Write};

use bytes::Bytes;
use chrono::{DateTime, Datelike, Utc};
use event::log::Value;
use event::{Event, LogRecord};
use log_schema::log_schema;
use lookup::path;
use smallvec::{smallvec, SmallVec};

use super::{DeserializeError, Deserializer};

/// Deserializer that builds on `Event` from a byte frame containing a syslog
/// message.
#[derive(Clone, Debug)]
pub struct SyslogDeserializer;

impl Deserializer for SyslogDeserializer {
    fn parse(&self, buf: Bytes) -> Result<SmallVec<[Event; 1]>, DeserializeError> {
        let line = std::str::from_utf8(&buf)?;
        let line = line.trim();
        let record = parse(line).map_err(DeserializeError::Syslog)?;

        Ok(smallvec![record.into()])
    }
}

#[derive(Debug)]
pub enum Error {
    BOM,
    Missing(&'static str),
    Timestamp(chrono::ParseError),
    Other(&'static str),
}

impl From<&'static str> for Error {
    fn from(value: &'static str) -> Self {
        Error::Other(value)
    }
}

impl From<chrono::ParseError> for Error {
    fn from(value: chrono::ParseError) -> Self {
        Error::Timestamp(value)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::BOM => f.write_str("Unsupported BOM"),
            Error::Missing(field) => write!(f, "missing {}", field),
            Error::Timestamp(err) => err.fmt(f),
            Error::Other(s) => f.write_str(s),
        }
    }
}

/// This syslog parse is copied from 'awslabs/flowgger', which is very simple and high performance.
///
/// https://github.com/awslabs/flowgger/blob/master/src/flowgger/decoder/rfc5424_decoder.rs
pub fn parse(msg: &str) -> Result<LogRecord, Error> {
    let (_bom, line) = match BOM::parse(msg, "<") {
        Some(bom_line) => bom_line,
        None => return Err(Error::BOM),
    };

    let mut parts = line.splitn(7, ' ');
    let pri_version = parse_pri_version(parts.next().ok_or(Error::Missing("pri and version"))?)?;
    let timestamp = DateTime::parse_from_rfc3339(parts.next().ok_or(Error::Missing("timestamp"))?)?;
    let hostname = parts.next().ok_or(Error::Missing("hostname"))?;
    let appname = parts.next().ok_or(Error::Missing("application name"))?;
    let procid = parts.next().ok_or(Error::Missing("process id"))?;
    let msgid = parts.next().ok_or(Error::Missing("message id"))?;
    let (sd_vec, msg) = parse_data(parts.next().ok_or(Error::Missing("message data"))?)?;

    let mut log = LogRecord::from(msg.unwrap_or_default());
    log.insert_field(log_schema().timestamp_key(), timestamp.with_timezone(&Utc));
    log.insert_field("hostname", hostname);
    log.insert_field("severity", severity_str(pri_version.severity));
    log.insert_field("facility", facility_str(pri_version.facility));
    log.insert_field("appname", appname);
    log.insert_field("msgid", msgid);
    match procid.parse::<i32>() {
        Ok(num) => log.insert_field("procid", num),
        Err(_) => log.insert_field("procid", procid),
    };

    for elmt in sd_vec {
        let mut map = BTreeMap::<String, Value>::new();

        for (name, value) in elmt.pairs {
            map.insert(name, value.into());
        }

        log.insert_field(path!(&elmt.name), map);
    }

    Ok(log)
}

struct Pri {
    facility: u8,
    severity: u8,
}

fn facility_str(facility: u8) -> &'static str {
    match facility {
        0 => "kern",
        1 => "user",
        2 => "mail",
        3 => "daemon",
        4 => "auth",
        5 => "syslog",
        6 => "lpr",
        7 => "news",
        8 => "uucp",
        9 => "cron",
        10 => "authpriv",
        11 => "ftp",
        12 => "ntp",
        13 => "audit",
        14 => "alert",
        15 => "clockd",
        16 => "local0",
        17 => "local1",
        18 => "local2",
        19 => "local3",
        20 => "local4",
        21 => "local5",
        22 => "local6",
        23 => "local7",
        _ => unreachable!(),
    }
}

fn severity_str(severity: u8) -> &'static str {
    match severity {
        0 => "emerg",
        1 => "alert",
        2 => "crit",
        3 => "err",
        4 => "warning",
        5 => "notice",
        6 => "info",
        7 => "debug",
        _ => unreachable!(),
    }
}

enum BOM {
    NONE,
    UTF8,
}

impl BOM {
    // None should be treat as error.
    fn parse<'a>(line: &'a str, sep: &str) -> Option<(BOM, &'a str)> {
        if line.starts_with('\u{feff}') {
            Some((BOM::UTF8, &line[3..]))
        } else if line.starts_with(sep) {
            Some((BOM::NONE, line))
        } else {
            None
        }
    }
}

fn parse_pri_version(line: &str) -> Result<Pri, &'static str> {
    if !line.starts_with('<') {
        return Err("The priority should be inside brackets");
    }
    let mut parts = line[1..].splitn(2, '>');
    let pri_encoded: u8 = parts
        .next()
        .ok_or("Empty priority")?
        .parse()
        .or(Err("Invalid priority"))?;
    let version = parts.next().ok_or("Missing version")?;
    if version != "1" {
        return Err("Unsupported version");
    }
    Ok(Pri {
        facility: pri_encoded >> 3,
        severity: pri_encoded & 7,
    })
}

fn unescape_sd_value(value: &str) -> String {
    let mut res = "".to_owned();
    let mut esc = false;

    for c in value.chars() {
        match (c, esc) {
            ('\\', false) => esc = true,
            (_, false) => res.push(c),
            ('"', true) | ('\\', true) | (']', true) => {
                res.push(c);
                esc = false;
            }
            (_, true) => {
                res.push('\\');
                res.push(c);
                esc = false;
            }
        }
    }
    res
}

pub struct StructuredData {
    pub name: String,
    pub pairs: Vec<(String, String)>,
}

impl StructuredData {
    pub fn new(sd_id: &str) -> StructuredData {
        StructuredData {
            name: sd_id.to_string(),
            pairs: Vec::new(),
        }
    }
}

fn parse_data(line: &str) -> Result<(Vec<StructuredData>, Option<String>), &'static str> {
    let mut sd_vec: Vec<StructuredData> = Vec::new();
    match line.chars().next().ok_or("Missing log message")? {
        '-' => {
            // No SD, just a message
            return Ok((sd_vec, parse_msg(line, 1)));
        }
        '[' => {
            // At least one SD
            let (mut leftover, mut offset) = (line, 0);
            let mut next_sd = true;
            while next_sd {
                let (sd, new_leftover, new_offset) = parse_sd_data(leftover, offset + 1)?;
                // Unfortunately we have to reassign, https://github.com/rust-lang/rfcs/pull/2909 not yet implemented
                leftover = new_leftover;
                offset = new_offset;
                sd_vec.push(sd);

                match leftover[offset..]
                    .chars()
                    .next()
                    .ok_or("Missing log message")?
                {
                    // Another SD
                    '[' => next_sd = true,
                    // Separator, the rest is the message
                    ' ' => return Ok((sd_vec, parse_msg(leftover, offset))),
                    _ => return Err("Malformated RFC5424 message"),
                }
            }
            return Ok((sd_vec, parse_msg(leftover, 1)));
        }
        _ => return Err("Malformated RFC5424 message"),
    };
}

fn parse_msg(line: &str, offset: usize) -> Option<String> {
    if offset > line.len() {
        None
    } else {
        match line[offset..].trim() {
            "" => None,
            m => Some(m.to_owned()),
        }
    }
}

fn parse_sd_data(line: &str, offset: usize) -> Result<(StructuredData, &str, usize), &'static str> {
    let mut parts = line[offset..].splitn(2, ' ');
    let sd_id = parts.next().ok_or("Missing structured data id")?;
    let sd = parts.next().ok_or("Missing structured data")?;
    let mut in_name = false;
    let mut in_value = false;
    let mut name_start = 0;
    let mut value_start = 0;
    let mut name: Option<&str> = None;
    let mut esc = false;
    let mut after_sd: Option<usize> = None;
    let mut sd_res = StructuredData::new(sd_id);

    for (i, c) in sd.char_indices() {
        let is_sd_name = match c as u32 {
            32 | 34 | 61 | 93 => false,
            33..=126 => true,
            _ => false,
        };
        match (c, esc, is_sd_name, in_name, name.is_some(), in_value) {
            (' ', false, _, false, false, _) => {
                // contextless spaces
            }
            (']', false, _, false, false, _) => {
                after_sd = Some(i + 1);
                break;
            }
            (_, false, true, false, false, _) => {
                in_name = true;
                name_start = i;
            }
            (_, _, true, true, false, _) => {
                // name
            }
            ('=', false, _, true, ..) => {
                name = Some(&sd[name_start..i]);
                in_name = false;
            }
            ('"', false, _, _, true, false) => {
                in_value = true;
                value_start = i + 1;
            }
            ('\\', false, _, _, _, true) => esc = true,
            ('"', false, _, _, _, true) => {
                in_value = false;
                let value = unescape_sd_value(&sd[value_start..i]);
                let pair = (
                    name.expect(
                        "Name in structured data contains an invalid UTF-8 \
                             sequence",
                    )
                    .to_string(),
                    value,
                );
                sd_res.pairs.push(pair);
                name = None;
            }
            (_, _, _, _, _, true) => esc = false,
            ('"', false, _, false, false, _) => {
                // tolerate bogus entries with extra "
            }
            _ => return Err("Format error in the structured data"),
        }
    }
    match after_sd {
        None => Err("Missing ] after structured data"),
        Some(offset) => Ok((sd_res, sd, offset)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use event::fields;

    #[test]
    fn test_rfc5424() {
        let msg = r#"<23>1 2015-08-05T15:53:45.637824Z testhostname appname 69 42 [origin@123 software="te\st sc\"ript" swVersion="0.0.1"] test message"#;
        let record = parse(msg).unwrap();

        let want = LogRecord::from(fields!(
            "facility" => facility_str(2),
            "severity" => severity_str(7),
            "timestamp" =>           DateTime::parse_from_rfc3339("2015-08-05T15:53:45.637824Z")
                .unwrap()
                .with_timezone(&Utc),
            "hostname" => "testhostname",
            "appname" => "appname",
            "procid" => 69,
            "msgid" => "42",
            "message" => "test message",
            "origin@123" => fields!(
                "software" => "te\\st sc\"ript",
                "swVersion" => "0.0.1"
            )
        ));

        assert_eq!(record, want);
    }

    #[test]
    fn test_rfc5424_multiple_sd() {
        let msg = r#"<23>1 2015-08-05T15:53:45.637824Z testhostname appname 69 42 [origin@123 software="te\st sc\"ript" swVersion="0.0.1"][master@456 key="value" key2="value2"] test message"#;
        let record = parse(msg).unwrap();

        let want = LogRecord::from(fields!(
            "facility" => facility_str(2),
            "severity" => severity_str(7),
            "timestamp" => DateTime::parse_from_rfc3339("2015-08-05T15:53:45.637824Z")
                .unwrap()
                .with_timezone(&Utc),
            "hostname" => "testhostname",
            "appname" => "appname",
            "procid" => 69,
            "msgid" => "42",
            "message" => "test message",
            "origin@123" => fields!(
                "software" => "te\\st sc\"ript",
                "swVersion" => "0.0.1"
            ),
            "master@456" => fields!(
                "key" => "value",
                "key2" => "value2"
            )
        ));

        assert_eq!(record, want);
    }
}
