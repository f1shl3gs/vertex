use std::collections::BTreeMap;

use bytes::Bytes;
use chrono::{DateTime, Datelike, Utc};
use event::log::Value;
use event::{Event, LogRecord};
use log_schema::log_schema;
use lookup::path;
use smallvec::{smallvec, SmallVec};
use syslog_loose::{IncompleteDate, Message, ProcId, Protocol, Variant};

use super::{DeserializeError, Deserializer};

/// Deserializer that builds on `Event` from a byte frame containing a syslog
/// message.
#[derive(Clone, Debug)]
pub struct SyslogDeserializer;

impl Deserializer for SyslogDeserializer {
    fn parse(&self, buf: Bytes) -> Result<SmallVec<[Event; 1]>, DeserializeError> {
        let line = std::str::from_utf8(&buf)?;
        let line = line.trim();
        let parsed =
            syslog_loose::parse_message_with_year_exact(line, resolve_year, Variant::Either)?;
        let mut event = LogRecord::from(parsed.msg).into();

        insert_fields_from_syslog(&mut event, parsed);

        Ok(smallvec![event])
    }
}

/// Function used to resolve the year for syslog messages that don't include the year.
///
/// If the current month is January, and the syslog message is for December, it will
/// take the previous year. Otherwise, take the current year.
fn resolve_year((month, _date, _hour, _min, _sec): IncompleteDate) -> i32 {
    let now = Utc::now();
    if now.month() == 1 && month == 12 {
        now.year() - 1
    } else {
        now.year()
    }
}

fn insert_fields_from_syslog(event: &mut Event, parsed: Message<&str>) {
    let log = event.as_mut_log();

    if let Some(ts) = parsed.timestamp {
        log.insert_field(log_schema().timestamp_key(), DateTime::<Utc>::from(ts));
    }

    if let Some(host) = parsed.hostname {
        log.insert_field("hostname", host.to_string());
    }

    if let Some(severity) = parsed.severity {
        log.insert_field("severity", severity.as_str().to_owned());
    }

    if let Some(facility) = parsed.facility {
        log.insert_field("facility", facility.as_str().to_owned());
    }

    if let Protocol::RFC5424(version) = parsed.protocol {
        log.insert_field("version", version as i64);
    }

    if let Some(app) = parsed.appname {
        log.insert_field("appname", app.to_owned());
    }

    if let Some(msg_id) = parsed.msgid {
        log.insert_field("msgid", msg_id.to_owned());
    }

    if let Some(procid) = parsed.procid {
        let value: Value = match procid {
            ProcId::PID(pid) => pid.into(),
            ProcId::Name(name) => name.to_string().into(),
        };

        log.insert_field("procid", value);
    }

    for elmt in parsed.structured_data.into_iter() {
        let mut structured = BTreeMap::<String, Value>::new();

        for (name, value) in elmt.params {
            structured.insert(name.to_string(), value.into());
        }
        log.insert_field(path!(elmt.id), structured);
    }
}
