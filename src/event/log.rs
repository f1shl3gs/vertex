use serde::{Deserialize};

use crate::event::value::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, PartialOrd, Deserialize)]
pub struct LogRecord {
    // time_unix_nano is the time when the event occurred
    pub time_unix_nano: u64,

    pub severity_number: i32,

    pub severity_text: Option<String>,

    // Short event identifier that does not contain varying parts. Name describes
    // what happened(e.g. "ProcessStarted"). Recommended to be no longer than 50
    // characters. Not guaranteed to be unique in any way
    pub name: Option<String>,

    pub body: Value,

    pub attributes: BTreeMap<String, String>,

    // flags, a bit field. 8 least significant bits are the trace flags as defined
    // in W3C Trace Context specification. 24 most significant bits are reserved
    // and must be set to 0. Readers must not assume that 24 most significant bits
    // will be zero and must correctly mask the bits when reading 8-bit trace flag
    // (use flags & TRACE_FLAGS_MASK)
    pub flags: u32,

    pub trace_id: Vec<u8>,

    pub span_id: Vec<u8>,
}