use std::borrow::Cow;
use std::collections::BTreeMap;

use bytes::Bytes;
use chrono::{DateTime, Datelike, Utc};
use configurable::Configurable;
use event::log::Value;
use event::{event_path, Event, LogRecord};
use log_schema::log_schema;
use serde::{Deserialize, Serialize};
use smallvec::{smallvec, SmallVec};
use syslog_loose::{IncompleteDate, Message, ProcId, Protocol, Variant};

use super::{DeserializeError, Deserializer};
use crate::serde::{default_lossy, skip_serializing_if_default};

/// Config used to build a `SyslogDeserializer`
#[derive(Clone, Configurable, Debug, Deserialize, Serialize)]
pub struct SyslogDeserializerConfig {
    /// Determines whether or not to replace invalid UTF-8 sequences instead of failing.
    ///
    /// When true, invalid UTF-8 sequences are replaced with the [`U+FFFD REPLACEMENT CHARACTER`][U+FFFD].
    ///
    /// [U+FFFD]: https://en.wikipedia.org/wiki/Specials_(Unicode_block)#Replacement_character
    #[serde(
        default = "default_lossy",
        skip_serializing_if = "skip_serializing_if_default"
    )]
    lossy: bool,
}

impl SyslogDeserializerConfig {
    /// Build the `SyslogDeserializer` from this configuration.
    #[inline]
    pub fn build(&self) -> SyslogDeserializer {
        SyslogDeserializer { lossy: self.lossy }
    }
}

/// Deserializer that builds on `Event` from a byte frame containing a syslog
/// message.
#[derive(Clone, Debug)]
pub struct SyslogDeserializer {
    lossy: bool,
}

impl Default for SyslogDeserializer {
    fn default() -> Self {
        Self {
            lossy: default_lossy(),
        }
    }
}

impl Deserializer for SyslogDeserializer {
    fn parse(&self, buf: Bytes) -> Result<SmallVec<[Event; 1]>, DeserializeError> {
        let line = if self.lossy {
            String::from_utf8_lossy(&buf)
        } else {
            Cow::from(std::str::from_utf8(&buf)?)
        };
        let parsed = syslog_loose::parse_message_with_year_exact(
            line.trim(),
            resolve_year,
            Variant::Either,
        )?;
        let event = convert(parsed).into();

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

fn convert(parsed: Message<&str>) -> LogRecord {
    let mut log = LogRecord::from(parsed.msg);

    if let Some(ts) = parsed.timestamp {
        log.insert(log_schema().timestamp_key(), DateTime::<Utc>::from(ts));
    }

    if let Some(host) = parsed.hostname {
        log.insert(event_path!("hostname"), host.to_string());
    }

    if let Some(severity) = parsed.severity {
        log.insert(event_path!("severity"), severity.as_str().to_owned());
    }

    if let Some(facility) = parsed.facility {
        log.insert(event_path!("facility"), facility.as_str().to_owned());
    }

    if let Protocol::RFC5424(version) = parsed.protocol {
        log.insert(event_path!("version"), version as i64);
    }

    if let Some(app) = parsed.appname {
        log.insert(event_path!("appname"), app.to_owned());
    }

    if let Some(msg_id) = parsed.msgid {
        log.insert(event_path!("msgid"), msg_id.to_owned());
    }

    if let Some(procid) = parsed.procid {
        let value: Value = match procid {
            ProcId::PID(pid) => Value::Integer(pid as i64),
            ProcId::Name(name) => name.to_string().into(),
        };

        log.insert(event_path!("procid"), value);
    }

    for elmt in parsed.structured_data.into_iter() {
        let mut structured = BTreeMap::<String, Value>::new();
        for (name, value) in elmt.params {
            structured.insert(name.to_string(), value.into());
        }

        log.insert(event_path!(elmt.id), structured);
    }

    log
}
