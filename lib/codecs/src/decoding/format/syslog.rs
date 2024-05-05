use std::borrow::Cow;
use std::collections::BTreeMap;

use bytes::Bytes;
use chrono::{DateTime, Utc};
use configurable::Configurable;
use event::log::Value;
use event::{event_path, Event, LogRecord};
use serde::{Deserialize, Serialize};
use smallvec::{smallvec, SmallVec};
use syslog::{ProcId, Protocol};

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

        let parsed =
            syslog::rfc5424::parse_message(line.as_bytes()).map_err(DeserializeError::Syslog)?;

        let mut log = LogRecord::from(parsed.msg);
        if let Some(ts) = parsed.timestamp {
            log.insert(event_path!("timestamp"), DateTime::<Utc>::from(ts));
        }

        if let Some(host) = parsed.hostname {
            log.insert(event_path!("hostname"), host);
        }

        log.insert(event_path!("severity"), parsed.severity.as_str());
        log.insert(event_path!("facility"), parsed.facility.as_str());

        if let Protocol::RFC5424(version) = parsed.protocol {
            log.insert(event_path!("version"), version);
        }

        if let Some(appname) = parsed.appname {
            log.insert(event_path!("appname"), appname);
        }

        if let Some(msg_id) = parsed.msgid {
            log.insert(event_path!("msgid"), msg_id);
        }

        if let Some(procid) = parsed.procid {
            let value: Value = match procid {
                ProcId::PID(pid) => Value::Integer(pid as i64),
                ProcId::Name(name) => name.into(),
            };

            log.insert(event_path!("procid"), value);
        }

        for element in parsed.structured_data {
            let mut structured = BTreeMap::<String, Value>::new();
            for (name, value) in element.params {
                structured.insert(name.to_string(), value.into());
            }

            log.insert(event_path!(element.id), structured);
        }

        Ok(smallvec![log.into()])
    }
}
