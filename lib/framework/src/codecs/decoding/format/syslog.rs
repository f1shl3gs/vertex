use bytes::Bytes;
use chrono::DateTime;
use chrono::{Datelike, Utc};
use event::{Event, Value};
use log_schema::log_schema;
use serde::{Deserialize, Serialize};
use smallvec::{smallvec, SmallVec};
use syslog_loose::{IncompleteDate, Message, ProcId, Protocol};

use super::Deserializer;

/// Config used to build a `SyslogDeserializer`
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct SyslogDeserializerConfig;

impl SyslogDeserializerConfig {
    pub fn build(&self) -> SyslogDeserializer {
        SyslogDeserializer
    }
}

/// Deserializer that builds an `Event` from a byte frame containing a
/// syslog message
#[derive(Debug, Clone)]
pub struct SyslogDeserializer;

impl Deserializer for SyslogDeserializer {
    fn parse(&self, bytes: Bytes) -> crate::Result<SmallVec<[Event; 1]>> {
        let line = std::str::from_utf8(&bytes).map_err(|err| {
            error!(
                message = "Error converting bytes to UTF-8 string",
                %err,
                internal_log_rate_secs = 10
            );

            err
        })?;

        let line = line.trim();
        let parsed = syslog_loose::parse_message_with_year(line, resolve_year);
        let mut event = Event::from(parsed.msg);

        insert_fields_from_syslog(&mut event, parsed);

        Ok(smallvec![event])
    }
}

/// Function used to resolve the year for syslog messages that don't include the year
///
/// If the current month is January, and the syslog message is for December, it
/// will take the previous year. Otherwise, take the current year
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
        for (name, value) in elmt.params {
            let key = format!("{}.{}", elmt.id, name);
            log.insert_field(key, value);
        }
    }
}
