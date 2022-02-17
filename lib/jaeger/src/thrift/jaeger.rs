// Autogenerated by Thrift Compiler (0.13.0)
// DO NOT EDIT UNLESS YOU ARE SURE THAT YOU KNOW WHAT YOU ARE DOING

#![allow(clippy::unused_unit)]
#![allow(unused_imports)]
#![allow(unused_extern_crates)]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::too_many_arguments, clippy::type_complexity))]
#![cfg_attr(rustfmt, rustfmt_skip)]

extern crate thrift;

use thrift::OrderedFloat;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::convert::{From, TryFrom};
use std::default::Default;
use std::error::Error;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::rc::Rc;

use thrift::{ApplicationError, ApplicationErrorKind, ProtocolError, ProtocolErrorKind, TThriftClient};
use thrift::protocol::{TFieldIdentifier, TListIdentifier, TMapIdentifier, TMessageIdentifier, TMessageType, TInputProtocol, TOutputProtocol, TSetIdentifier, TStructIdentifier, TType};
use thrift::protocol::field_id;
use thrift::protocol::verify_expected_message_type;
use thrift::protocol::verify_expected_sequence_number;
use thrift::protocol::verify_expected_service_call;
use thrift::protocol::verify_required_field_exists;
use thrift::server::TProcessor;

#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum TagType {
  String = 0,
  Double = 1,
  Bool = 2,
  Long = 3,
  Binary = 4,
}

impl TagType {
  pub fn write_to_out_protocol(&self, o_prot: &mut dyn TOutputProtocol) -> thrift::Result<()> {
    o_prot.write_i32(*self as i32)
  }
  pub fn read_from_in_protocol(i_prot: &mut dyn TInputProtocol) -> thrift::Result<TagType> {
    let enum_value = i_prot.read_i32()?;
    TagType::try_from(enum_value)  }
}

impl TryFrom<i32> for TagType {
  type Error = thrift::Error;  fn try_from(i: i32) -> Result<Self, Self::Error> {
    match i {
      0 => Ok(TagType::String),
      1 => Ok(TagType::Double),
      2 => Ok(TagType::Bool),
      3 => Ok(TagType::Long),
      4 => Ok(TagType::Binary),
      _ => {
        Err(
          thrift::Error::Protocol(
            ProtocolError::new(
              ProtocolErrorKind::InvalidData,
              format!("cannot convert enum constant {} to TagType", i)
            )
          )
        )
      },
    }
  }
}

#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SpanRefType {
  ChildOf = 0,
  FollowsFrom = 1,
}

impl SpanRefType {
  pub fn write_to_out_protocol(&self, o_prot: &mut dyn TOutputProtocol) -> thrift::Result<()> {
    o_prot.write_i32(*self as i32)
  }
  pub fn read_from_in_protocol(i_prot: &mut dyn TInputProtocol) -> thrift::Result<SpanRefType> {
    let enum_value = i_prot.read_i32()?;
    SpanRefType::try_from(enum_value)  }
}

impl TryFrom<i32> for SpanRefType {
  type Error = thrift::Error;  fn try_from(i: i32) -> Result<Self, Self::Error> {
    match i {
      0 => Ok(SpanRefType::ChildOf),
      1 => Ok(SpanRefType::FollowsFrom),
      _ => {
        Err(
          thrift::Error::Protocol(
            ProtocolError::new(
              ProtocolErrorKind::InvalidData,
              format!("cannot convert enum constant {} to SpanRefType", i)
            )
          )
        )
      },
    }
  }
}

//
// Tag
//

#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Tag {
  pub key: String,
  pub v_type: TagType,
  pub v_str: Option<String>,
  pub v_double: Option<OrderedFloat<f64>>,
  pub v_bool: Option<bool>,
  pub v_long: Option<i64>,
  pub v_binary: Option<Vec<u8>>,
}

impl Tag {
  pub fn new<F3, F4, F5, F6, F7>(key: String, v_type: TagType, v_str: F3, v_double: F4, v_bool: F5, v_long: F6, v_binary: F7) -> Tag where F3: Into<Option<String>>, F4: Into<Option<OrderedFloat<f64>>>, F5: Into<Option<bool>>, F6: Into<Option<i64>>, F7: Into<Option<Vec<u8>>> {
    Tag {
      key,
      v_type,
      v_str: v_str.into(),
      v_double: v_double.into(),
      v_bool: v_bool.into(),
      v_long: v_long.into(),
      v_binary: v_binary.into(),
    }
  }
  pub fn read_from_in_protocol(i_prot: &mut dyn TInputProtocol) -> thrift::Result<Tag> {
    i_prot.read_struct_begin()?;
    let mut f_1: Option<String> = None;
    let mut f_2: Option<TagType> = None;
    let mut f_3: Option<String> = None;
    let mut f_4: Option<OrderedFloat<f64>> = None;
    let mut f_5: Option<bool> = None;
    let mut f_6: Option<i64> = None;
    let mut f_7: Option<Vec<u8>> = None;
    loop {
      let field_ident = i_prot.read_field_begin()?;
      if field_ident.field_type == TType::Stop {
        break;
      }
      let field_id = field_id(&field_ident)?;
      match field_id {
        1 => {
          let val = i_prot.read_string()?;
          f_1 = Some(val);
        },
        2 => {
          let val = TagType::read_from_in_protocol(i_prot)?;
          f_2 = Some(val);
        },
        3 => {
          let val = i_prot.read_string()?;
          f_3 = Some(val);
        },
        4 => {
          let val = OrderedFloat::from(i_prot.read_double()?);
          f_4 = Some(val);
        },
        5 => {
          let val = i_prot.read_bool()?;
          f_5 = Some(val);
        },
        6 => {
          let val = i_prot.read_i64()?;
          f_6 = Some(val);
        },
        7 => {
          let val = i_prot.read_bytes()?;
          f_7 = Some(val);
        },
        _ => {
          i_prot.skip(field_ident.field_type)?;
        },
      };
      i_prot.read_field_end()?;
    }
    i_prot.read_struct_end()?;
    verify_required_field_exists("Tag.key", &f_1)?;
    verify_required_field_exists("Tag.v_type", &f_2)?;
    let ret = Tag {
      key: f_1.expect("auto-generated code should have checked for presence of required fields"),
      v_type: f_2.expect("auto-generated code should have checked for presence of required fields"),
      v_str: f_3,
      v_double: f_4,
      v_bool: f_5,
      v_long: f_6,
      v_binary: f_7,
    };
    Ok(ret)
  }
  pub fn write_to_out_protocol(&self, o_prot: &mut dyn TOutputProtocol) -> thrift::Result<()> {
    let struct_ident = TStructIdentifier::new("Tag");
    o_prot.write_struct_begin(&struct_ident)?;
    o_prot.write_field_begin(&TFieldIdentifier::new("key", TType::String, 1))?;
    o_prot.write_string(&self.key)?;
    o_prot.write_field_end()?;
    o_prot.write_field_begin(&TFieldIdentifier::new("vType", TType::I32, 2))?;
    self.v_type.write_to_out_protocol(o_prot)?;
    o_prot.write_field_end()?;
    if let Some(ref fld_var) = self.v_str {
      o_prot.write_field_begin(&TFieldIdentifier::new("vStr", TType::String, 3))?;
      o_prot.write_string(fld_var)?;
      o_prot.write_field_end()?;
      ()
    } else {
      ()
    }
    if let Some(fld_var) = self.v_double {
      o_prot.write_field_begin(&TFieldIdentifier::new("vDouble", TType::Double, 4))?;
      o_prot.write_double(fld_var.into())?;
      o_prot.write_field_end()?;
      ()
    } else {
      ()
    }
    if let Some(fld_var) = self.v_bool {
      o_prot.write_field_begin(&TFieldIdentifier::new("vBool", TType::Bool, 5))?;
      o_prot.write_bool(fld_var)?;
      o_prot.write_field_end()?;
      ()
    } else {
      ()
    }
    if let Some(fld_var) = self.v_long {
      o_prot.write_field_begin(&TFieldIdentifier::new("vLong", TType::I64, 6))?;
      o_prot.write_i64(fld_var)?;
      o_prot.write_field_end()?;
      ()
    } else {
      ()
    }
    if let Some(ref fld_var) = self.v_binary {
      o_prot.write_field_begin(&TFieldIdentifier::new("vBinary", TType::String, 7))?;
      o_prot.write_bytes(fld_var)?;
      o_prot.write_field_end()?;
      ()
    } else {
      ()
    }
    o_prot.write_field_stop()?;
    o_prot.write_struct_end()
  }
}

//
// Log
//

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Log {
  pub timestamp: i64,
  pub fields: Vec<Tag>,
}

impl Log {
  pub fn new(timestamp: i64, fields: Vec<Tag>) -> Log {
    Log {
      timestamp,
      fields,
    }
  }
  pub fn read_from_in_protocol(i_prot: &mut dyn TInputProtocol) -> thrift::Result<Log> {
    i_prot.read_struct_begin()?;
    let mut f_1: Option<i64> = None;
    let mut f_2: Option<Vec<Tag>> = None;
    loop {
      let field_ident = i_prot.read_field_begin()?;
      if field_ident.field_type == TType::Stop {
        break;
      }
      let field_id = field_id(&field_ident)?;
      match field_id {
        1 => {
          let val = i_prot.read_i64()?;
          f_1 = Some(val);
        },
        2 => {
          let list_ident = i_prot.read_list_begin()?;
          let mut val: Vec<Tag> = Vec::with_capacity(list_ident.size as usize);
          for _ in 0..list_ident.size {
            let list_elem_0 = Tag::read_from_in_protocol(i_prot)?;
            val.push(list_elem_0);
          }
          i_prot.read_list_end()?;
          f_2 = Some(val);
        },
        _ => {
          i_prot.skip(field_ident.field_type)?;
        },
      };
      i_prot.read_field_end()?;
    }
    i_prot.read_struct_end()?;
    verify_required_field_exists("Log.timestamp", &f_1)?;
    verify_required_field_exists("Log.fields", &f_2)?;
    let ret = Log {
      timestamp: f_1.expect("auto-generated code should have checked for presence of required fields"),
      fields: f_2.expect("auto-generated code should have checked for presence of required fields"),
    };
    Ok(ret)
  }
  pub fn write_to_out_protocol(&self, o_prot: &mut dyn TOutputProtocol) -> thrift::Result<()> {
    let struct_ident = TStructIdentifier::new("Log");
    o_prot.write_struct_begin(&struct_ident)?;
    o_prot.write_field_begin(&TFieldIdentifier::new("timestamp", TType::I64, 1))?;
    o_prot.write_i64(self.timestamp)?;
    o_prot.write_field_end()?;
    o_prot.write_field_begin(&TFieldIdentifier::new("fields", TType::List, 2))?;
    o_prot.write_list_begin(&TListIdentifier::new(TType::Struct, self.fields.len() as i32))?;
    for e in &self.fields {
      e.write_to_out_protocol(o_prot)?;
      o_prot.write_list_end()?;
    }
    o_prot.write_field_end()?;
    o_prot.write_field_stop()?;
    o_prot.write_struct_end()
  }
}

//
// SpanRef
//

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SpanRef {
  pub ref_type: SpanRefType,
  pub trace_id_low: i64,
  pub trace_id_high: i64,
  pub span_id: i64,
}

impl SpanRef {
  pub fn new(ref_type: SpanRefType, trace_id_low: i64, trace_id_high: i64, span_id: i64) -> SpanRef {
    SpanRef {
      ref_type,
      trace_id_low,
      trace_id_high,
      span_id,
    }
  }
  pub fn read_from_in_protocol(i_prot: &mut dyn TInputProtocol) -> thrift::Result<SpanRef> {
    i_prot.read_struct_begin()?;
    let mut f_1: Option<SpanRefType> = None;
    let mut f_2: Option<i64> = None;
    let mut f_3: Option<i64> = None;
    let mut f_4: Option<i64> = None;
    loop {
      let field_ident = i_prot.read_field_begin()?;
      if field_ident.field_type == TType::Stop {
        break;
      }
      let field_id = field_id(&field_ident)?;
      match field_id {
        1 => {
          let val = SpanRefType::read_from_in_protocol(i_prot)?;
          f_1 = Some(val);
        },
        2 => {
          let val = i_prot.read_i64()?;
          f_2 = Some(val);
        },
        3 => {
          let val = i_prot.read_i64()?;
          f_3 = Some(val);
        },
        4 => {
          let val = i_prot.read_i64()?;
          f_4 = Some(val);
        },
        _ => {
          i_prot.skip(field_ident.field_type)?;
        },
      };
      i_prot.read_field_end()?;
    }
    i_prot.read_struct_end()?;
    verify_required_field_exists("SpanRef.ref_type", &f_1)?;
    verify_required_field_exists("SpanRef.trace_id_low", &f_2)?;
    verify_required_field_exists("SpanRef.trace_id_high", &f_3)?;
    verify_required_field_exists("SpanRef.span_id", &f_4)?;
    let ret = SpanRef {
      ref_type: f_1.expect("auto-generated code should have checked for presence of required fields"),
      trace_id_low: f_2.expect("auto-generated code should have checked for presence of required fields"),
      trace_id_high: f_3.expect("auto-generated code should have checked for presence of required fields"),
      span_id: f_4.expect("auto-generated code should have checked for presence of required fields"),
    };
    Ok(ret)
  }
  pub fn write_to_out_protocol(&self, o_prot: &mut dyn TOutputProtocol) -> thrift::Result<()> {
    let struct_ident = TStructIdentifier::new("SpanRef");
    o_prot.write_struct_begin(&struct_ident)?;
    o_prot.write_field_begin(&TFieldIdentifier::new("refType", TType::I32, 1))?;
    self.ref_type.write_to_out_protocol(o_prot)?;
    o_prot.write_field_end()?;
    o_prot.write_field_begin(&TFieldIdentifier::new("traceIdLow", TType::I64, 2))?;
    o_prot.write_i64(self.trace_id_low)?;
    o_prot.write_field_end()?;
    o_prot.write_field_begin(&TFieldIdentifier::new("traceIdHigh", TType::I64, 3))?;
    o_prot.write_i64(self.trace_id_high)?;
    o_prot.write_field_end()?;
    o_prot.write_field_begin(&TFieldIdentifier::new("spanId", TType::I64, 4))?;
    o_prot.write_i64(self.span_id)?;
    o_prot.write_field_end()?;
    o_prot.write_field_stop()?;
    o_prot.write_struct_end()
  }
}

//
// Span
//

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Span {
  pub trace_id_low: i64,
  pub trace_id_high: i64,
  pub span_id: i64,
  pub parent_span_id: i64,
  pub operation_name: String,
  pub references: Option<Vec<SpanRef>>,
  pub flags: i32,
  pub start_time: i64,
  pub duration: i64,
  pub tags: Option<Vec<Tag>>,
  pub logs: Option<Vec<Log>>,
}

impl Span {
  pub fn new<F6, F10, F11>(trace_id_low: i64, trace_id_high: i64, span_id: i64, parent_span_id: i64, operation_name: String, references: F6, flags: i32, start_time: i64, duration: i64, tags: F10, logs: F11) -> Span where F6: Into<Option<Vec<SpanRef>>>, F10: Into<Option<Vec<Tag>>>, F11: Into<Option<Vec<Log>>> {
    Span {
      trace_id_low,
      trace_id_high,
      span_id,
      parent_span_id,
      operation_name,
      references: references.into(),
      flags,
      start_time,
      duration,
      tags: tags.into(),
      logs: logs.into(),
    }
  }
  pub fn read_from_in_protocol(i_prot: &mut dyn TInputProtocol) -> thrift::Result<Span> {
    i_prot.read_struct_begin()?;
    let mut f_1: Option<i64> = None;
    let mut f_2: Option<i64> = None;
    let mut f_3: Option<i64> = None;
    let mut f_4: Option<i64> = None;
    let mut f_5: Option<String> = None;
    let mut f_6: Option<Vec<SpanRef>> = None;
    let mut f_7: Option<i32> = None;
    let mut f_8: Option<i64> = None;
    let mut f_9: Option<i64> = None;
    let mut f_10: Option<Vec<Tag>> = None;
    let mut f_11: Option<Vec<Log>> = None;
    loop {
      let field_ident = i_prot.read_field_begin()?;
      if field_ident.field_type == TType::Stop {
        break;
      }
      let field_id = field_id(&field_ident)?;
      match field_id {
        1 => {
          let val = i_prot.read_i64()?;
          f_1 = Some(val);
        },
        2 => {
          let val = i_prot.read_i64()?;
          f_2 = Some(val);
        },
        3 => {
          let val = i_prot.read_i64()?;
          f_3 = Some(val);
        },
        4 => {
          let val = i_prot.read_i64()?;
          f_4 = Some(val);
        },
        5 => {
          let val = i_prot.read_string()?;
          f_5 = Some(val);
        },
        6 => {
          let list_ident = i_prot.read_list_begin()?;
          let mut val: Vec<SpanRef> = Vec::with_capacity(list_ident.size as usize);
          for _ in 0..list_ident.size {
            let list_elem_1 = SpanRef::read_from_in_protocol(i_prot)?;
            val.push(list_elem_1);
          }
          i_prot.read_list_end()?;
          f_6 = Some(val);
        },
        7 => {
          let val = i_prot.read_i32()?;
          f_7 = Some(val);
        },
        8 => {
          let val = i_prot.read_i64()?;
          f_8 = Some(val);
        },
        9 => {
          let val = i_prot.read_i64()?;
          f_9 = Some(val);
        },
        10 => {
          let list_ident = i_prot.read_list_begin()?;
          let mut val: Vec<Tag> = Vec::with_capacity(list_ident.size as usize);
          for _ in 0..list_ident.size {
            let list_elem_2 = Tag::read_from_in_protocol(i_prot)?;
            val.push(list_elem_2);
          }
          i_prot.read_list_end()?;
          f_10 = Some(val);
        },
        11 => {
          let list_ident = i_prot.read_list_begin()?;
          let mut val: Vec<Log> = Vec::with_capacity(list_ident.size as usize);
          for _ in 0..list_ident.size {
            let list_elem_3 = Log::read_from_in_protocol(i_prot)?;
            val.push(list_elem_3);
          }
          i_prot.read_list_end()?;
          f_11 = Some(val);
        },
        _ => {
          i_prot.skip(field_ident.field_type)?;
        },
      };
      i_prot.read_field_end()?;
    }
    i_prot.read_struct_end()?;
    verify_required_field_exists("Span.trace_id_low", &f_1)?;
    verify_required_field_exists("Span.trace_id_high", &f_2)?;
    verify_required_field_exists("Span.span_id", &f_3)?;
    verify_required_field_exists("Span.parent_span_id", &f_4)?;
    verify_required_field_exists("Span.operation_name", &f_5)?;
    verify_required_field_exists("Span.flags", &f_7)?;
    verify_required_field_exists("Span.start_time", &f_8)?;
    verify_required_field_exists("Span.duration", &f_9)?;
    let ret = Span {
      trace_id_low: f_1.expect("auto-generated code should have checked for presence of required fields"),
      trace_id_high: f_2.expect("auto-generated code should have checked for presence of required fields"),
      span_id: f_3.expect("auto-generated code should have checked for presence of required fields"),
      parent_span_id: f_4.expect("auto-generated code should have checked for presence of required fields"),
      operation_name: f_5.expect("auto-generated code should have checked for presence of required fields"),
      references: f_6,
      flags: f_7.expect("auto-generated code should have checked for presence of required fields"),
      start_time: f_8.expect("auto-generated code should have checked for presence of required fields"),
      duration: f_9.expect("auto-generated code should have checked for presence of required fields"),
      tags: f_10,
      logs: f_11,
    };
    Ok(ret)
  }
  pub fn write_to_out_protocol(&self, o_prot: &mut dyn TOutputProtocol) -> thrift::Result<()> {
    let struct_ident = TStructIdentifier::new("Span");
    o_prot.write_struct_begin(&struct_ident)?;
    o_prot.write_field_begin(&TFieldIdentifier::new("traceIdLow", TType::I64, 1))?;
    o_prot.write_i64(self.trace_id_low)?;
    o_prot.write_field_end()?;
    o_prot.write_field_begin(&TFieldIdentifier::new("traceIdHigh", TType::I64, 2))?;
    o_prot.write_i64(self.trace_id_high)?;
    o_prot.write_field_end()?;
    o_prot.write_field_begin(&TFieldIdentifier::new("spanId", TType::I64, 3))?;
    o_prot.write_i64(self.span_id)?;
    o_prot.write_field_end()?;
    o_prot.write_field_begin(&TFieldIdentifier::new("parentSpanId", TType::I64, 4))?;
    o_prot.write_i64(self.parent_span_id)?;
    o_prot.write_field_end()?;
    o_prot.write_field_begin(&TFieldIdentifier::new("operationName", TType::String, 5))?;
    o_prot.write_string(&self.operation_name)?;
    o_prot.write_field_end()?;
    if let Some(ref fld_var) = self.references {
      o_prot.write_field_begin(&TFieldIdentifier::new("references", TType::List, 6))?;
      o_prot.write_list_begin(&TListIdentifier::new(TType::Struct, fld_var.len() as i32))?;
      for e in fld_var {
        e.write_to_out_protocol(o_prot)?;
        o_prot.write_list_end()?;
      }
      o_prot.write_field_end()?;
      ()
    } else {
      ()
    }
    o_prot.write_field_begin(&TFieldIdentifier::new("flags", TType::I32, 7))?;
    o_prot.write_i32(self.flags)?;
    o_prot.write_field_end()?;
    o_prot.write_field_begin(&TFieldIdentifier::new("startTime", TType::I64, 8))?;
    o_prot.write_i64(self.start_time)?;
    o_prot.write_field_end()?;
    o_prot.write_field_begin(&TFieldIdentifier::new("duration", TType::I64, 9))?;
    o_prot.write_i64(self.duration)?;
    o_prot.write_field_end()?;
    if let Some(ref fld_var) = self.tags {
      o_prot.write_field_begin(&TFieldIdentifier::new("tags", TType::List, 10))?;
      o_prot.write_list_begin(&TListIdentifier::new(TType::Struct, fld_var.len() as i32))?;
      for e in fld_var {
        e.write_to_out_protocol(o_prot)?;
        o_prot.write_list_end()?;
      }
      o_prot.write_field_end()?;
      ()
    } else {
      ()
    }
    if let Some(ref fld_var) = self.logs {
      o_prot.write_field_begin(&TFieldIdentifier::new("logs", TType::List, 11))?;
      o_prot.write_list_begin(&TListIdentifier::new(TType::Struct, fld_var.len() as i32))?;
      for e in fld_var {
        e.write_to_out_protocol(o_prot)?;
        o_prot.write_list_end()?;
      }
      o_prot.write_field_end()?;
      ()
    } else {
      ()
    }
    o_prot.write_field_stop()?;
    o_prot.write_struct_end()
  }
}

//
// Process
//

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Process {
  pub service_name: String,
  pub tags: Option<Vec<Tag>>,
}

impl Process {
  pub fn new<F2>(service_name: String, tags: F2) -> Process where F2: Into<Option<Vec<Tag>>> {
    Process {
      service_name,
      tags: tags.into(),
    }
  }
  pub fn read_from_in_protocol(i_prot: &mut dyn TInputProtocol) -> thrift::Result<Process> {
    i_prot.read_struct_begin()?;
    let mut f_1: Option<String> = None;
    let mut f_2: Option<Vec<Tag>> = None;
    loop {
      let field_ident = i_prot.read_field_begin()?;
      if field_ident.field_type == TType::Stop {
        break;
      }
      let field_id = field_id(&field_ident)?;
      match field_id {
        1 => {
          let val = i_prot.read_string()?;
          f_1 = Some(val);
        },
        2 => {
          let list_ident = i_prot.read_list_begin()?;
          let mut val: Vec<Tag> = Vec::with_capacity(list_ident.size as usize);
          for _ in 0..list_ident.size {
            let list_elem_4 = Tag::read_from_in_protocol(i_prot)?;
            val.push(list_elem_4);
          }
          i_prot.read_list_end()?;
          f_2 = Some(val);
        },
        _ => {
          i_prot.skip(field_ident.field_type)?;
        },
      };
      i_prot.read_field_end()?;
    }
    i_prot.read_struct_end()?;
    verify_required_field_exists("Process.service_name", &f_1)?;
    let ret = Process {
      service_name: f_1.expect("auto-generated code should have checked for presence of required fields"),
      tags: f_2,
    };
    Ok(ret)
  }
  pub fn write_to_out_protocol(&self, o_prot: &mut dyn TOutputProtocol) -> thrift::Result<()> {
    let struct_ident = TStructIdentifier::new("Process");
    o_prot.write_struct_begin(&struct_ident)?;
    o_prot.write_field_begin(&TFieldIdentifier::new("serviceName", TType::String, 1))?;
    o_prot.write_string(&self.service_name)?;
    o_prot.write_field_end()?;
    if let Some(ref fld_var) = self.tags {
      o_prot.write_field_begin(&TFieldIdentifier::new("tags", TType::List, 2))?;
      o_prot.write_list_begin(&TListIdentifier::new(TType::Struct, fld_var.len() as i32))?;
      for e in fld_var {
        e.write_to_out_protocol(o_prot)?;
        o_prot.write_list_end()?;
      }
      o_prot.write_field_end()?;
      ()
    } else {
      ()
    }
    o_prot.write_field_stop()?;
    o_prot.write_struct_end()
  }
}

//
// Batch
//

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Batch {
  pub process: Process,
  pub spans: Vec<Span>,
}

impl Batch {
  pub fn new(process: Process, spans: Vec<Span>) -> Batch {
    Batch {
      process,
      spans,
    }
  }
  pub fn read_from_in_protocol(i_prot: &mut dyn TInputProtocol) -> thrift::Result<Batch> {
    i_prot.read_struct_begin()?;
    let mut f_1: Option<Process> = None;
    let mut f_2: Option<Vec<Span>> = None;
    loop {
      let field_ident = i_prot.read_field_begin()?;
      if field_ident.field_type == TType::Stop {
        break;
      }
      let field_id = field_id(&field_ident)?;
      match field_id {
        1 => {
          let val = Process::read_from_in_protocol(i_prot)?;
          f_1 = Some(val);
        },
        2 => {
          let list_ident = i_prot.read_list_begin()?;
          let mut val: Vec<Span> = Vec::with_capacity(list_ident.size as usize);
          for _ in 0..list_ident.size {
            let list_elem_5 = Span::read_from_in_protocol(i_prot)?;
            val.push(list_elem_5);
          }
          i_prot.read_list_end()?;
          f_2 = Some(val);
        },
        _ => {
          i_prot.skip(field_ident.field_type)?;
        },
      };
      i_prot.read_field_end()?;
    }
    i_prot.read_struct_end()?;
    verify_required_field_exists("Batch.process", &f_1)?;
    verify_required_field_exists("Batch.spans", &f_2)?;
    let ret = Batch {
      process: f_1.expect("auto-generated code should have checked for presence of required fields"),
      spans: f_2.expect("auto-generated code should have checked for presence of required fields"),
    };
    Ok(ret)
  }
  pub fn write_to_out_protocol(&self, o_prot: &mut dyn TOutputProtocol) -> thrift::Result<()> {
    let struct_ident = TStructIdentifier::new("Batch");
    o_prot.write_struct_begin(&struct_ident)?;
    o_prot.write_field_begin(&TFieldIdentifier::new("process", TType::Struct, 1))?;
    self.process.write_to_out_protocol(o_prot)?;
    o_prot.write_field_end()?;
    o_prot.write_field_begin(&TFieldIdentifier::new("spans", TType::List, 2))?;
    o_prot.write_list_begin(&TListIdentifier::new(TType::Struct, self.spans.len() as i32))?;
    for e in &self.spans {
      e.write_to_out_protocol(o_prot)?;
      o_prot.write_list_end()?;
    }
    o_prot.write_field_end()?;
    o_prot.write_field_stop()?;
    o_prot.write_struct_end()
  }
}

//
// BatchSubmitResponse
//

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BatchSubmitResponse {
  pub ok: bool,
}

impl BatchSubmitResponse {
  pub fn new(ok: bool) -> BatchSubmitResponse {
    BatchSubmitResponse {
      ok,
    }
  }
  pub fn read_from_in_protocol(i_prot: &mut dyn TInputProtocol) -> thrift::Result<BatchSubmitResponse> {
    i_prot.read_struct_begin()?;
    let mut f_1: Option<bool> = None;
    loop {
      let field_ident = i_prot.read_field_begin()?;
      if field_ident.field_type == TType::Stop {
        break;
      }
      let field_id = field_id(&field_ident)?;
      match field_id {
        1 => {
          let val = i_prot.read_bool()?;
          f_1 = Some(val);
        },
        _ => {
          i_prot.skip(field_ident.field_type)?;
        },
      };
      i_prot.read_field_end()?;
    }
    i_prot.read_struct_end()?;
    verify_required_field_exists("BatchSubmitResponse.ok", &f_1)?;
    let ret = BatchSubmitResponse {
      ok: f_1.expect("auto-generated code should have checked for presence of required fields"),
    };
    Ok(ret)
  }
  pub fn write_to_out_protocol(&self, o_prot: &mut dyn TOutputProtocol) -> thrift::Result<()> {
    let struct_ident = TStructIdentifier::new("BatchSubmitResponse");
    o_prot.write_struct_begin(&struct_ident)?;
    o_prot.write_field_begin(&TFieldIdentifier::new("ok", TType::Bool, 1))?;
    o_prot.write_bool(self.ok)?;
    o_prot.write_field_end()?;
    o_prot.write_field_stop()?;
    o_prot.write_struct_end()
  }
}

//
// Collector service client
//

pub trait TCollectorSyncClient {
  fn submit_batches(&mut self, batches: Vec<Batch>) -> thrift::Result<Vec<BatchSubmitResponse>>;
}

pub trait TCollectorSyncClientMarker {}

pub struct CollectorSyncClient<IP, OP> where IP: TInputProtocol, OP: TOutputProtocol {
  _i_prot: IP,
  _o_prot: OP,
  _sequence_number: i32,
}

impl <IP, OP> CollectorSyncClient<IP, OP> where IP: TInputProtocol, OP: TOutputProtocol {
  pub fn new(input_protocol: IP, output_protocol: OP) -> CollectorSyncClient<IP, OP> {
    CollectorSyncClient { _i_prot: input_protocol, _o_prot: output_protocol, _sequence_number: 0 }
  }
}

impl <IP, OP> TThriftClient for CollectorSyncClient<IP, OP> where IP: TInputProtocol, OP: TOutputProtocol {
  fn i_prot_mut(&mut self) -> &mut dyn TInputProtocol { &mut self._i_prot }
  fn o_prot_mut(&mut self) -> &mut dyn TOutputProtocol { &mut self._o_prot }
  fn sequence_number(&self) -> i32 { self._sequence_number }
  fn increment_sequence_number(&mut self) -> i32 { self._sequence_number += 1; self._sequence_number }
}

impl <IP, OP> TCollectorSyncClientMarker for CollectorSyncClient<IP, OP> where IP: TInputProtocol, OP: TOutputProtocol {}

impl <C: TThriftClient + TCollectorSyncClientMarker> TCollectorSyncClient for C {
  fn submit_batches(&mut self, batches: Vec<Batch>) -> thrift::Result<Vec<BatchSubmitResponse>> {
    (
      {
        self.increment_sequence_number();
        let message_ident = TMessageIdentifier::new("submitBatches", TMessageType::Call, self.sequence_number());
        let call_args = CollectorSubmitBatchesArgs { batches };
        self.o_prot_mut().write_message_begin(&message_ident)?;
        call_args.write_to_out_protocol(self.o_prot_mut())?;
        self.o_prot_mut().write_message_end()?;
        self.o_prot_mut().flush()
      }
    )?;
    {
      let message_ident = self.i_prot_mut().read_message_begin()?;
      verify_expected_sequence_number(self.sequence_number(), message_ident.sequence_number)?;
      verify_expected_service_call("submitBatches", &message_ident.name)?;
      if message_ident.message_type == TMessageType::Exception {
        let remote_error = thrift::Error::read_application_error_from_in_protocol(self.i_prot_mut())?;
        self.i_prot_mut().read_message_end()?;
        return Err(thrift::Error::Application(remote_error))
      }
      verify_expected_message_type(TMessageType::Reply, message_ident.message_type)?;
      let result = CollectorSubmitBatchesResult::read_from_in_protocol(self.i_prot_mut())?;
      self.i_prot_mut().read_message_end()?;
      result.ok_or()
    }
  }
}

//
// Collector service processor
//

pub trait CollectorSyncHandler {
  fn handle_submit_batches(&self, batches: Vec<Batch>) -> thrift::Result<Vec<BatchSubmitResponse>>;
}

pub struct CollectorSyncProcessor<H: CollectorSyncHandler> {
  handler: H,
}

impl <H: CollectorSyncHandler> CollectorSyncProcessor<H> {
  pub fn new(handler: H) -> CollectorSyncProcessor<H> {
    CollectorSyncProcessor {
      handler,
    }
  }
  fn process_submit_batches(&self, incoming_sequence_number: i32, i_prot: &mut dyn TInputProtocol, o_prot: &mut dyn TOutputProtocol) -> thrift::Result<()> {
    TCollectorProcessFunctions::process_submit_batches(&self.handler, incoming_sequence_number, i_prot, o_prot)
  }
}

pub struct TCollectorProcessFunctions;

impl TCollectorProcessFunctions {
  pub fn process_submit_batches<H: CollectorSyncHandler>(handler: &H, incoming_sequence_number: i32, i_prot: &mut dyn TInputProtocol, o_prot: &mut dyn TOutputProtocol) -> thrift::Result<()> {
    let args = CollectorSubmitBatchesArgs::read_from_in_protocol(i_prot)?;
    match handler.handle_submit_batches(args.batches) {
      Ok(handler_return) => {
        let message_ident = TMessageIdentifier::new("submitBatches", TMessageType::Reply, incoming_sequence_number);
        o_prot.write_message_begin(&message_ident)?;
        let ret = CollectorSubmitBatchesResult { result_value: Some(handler_return) };
        ret.write_to_out_protocol(o_prot)?;
        o_prot.write_message_end()?;
        o_prot.flush()
      },
      Err(e) => {
        match e {
          thrift::Error::Application(app_err) => {
            let message_ident = TMessageIdentifier::new("submitBatches", TMessageType::Exception, incoming_sequence_number);
            o_prot.write_message_begin(&message_ident)?;
            thrift::Error::write_application_error_to_out_protocol(&app_err, o_prot)?;
            o_prot.write_message_end()?;
            o_prot.flush()
          },
          _ => {
            let ret_err = {
              ApplicationError::new(
                ApplicationErrorKind::Unknown,
                e.to_string()
              )
            };
            let message_ident = TMessageIdentifier::new("submitBatches", TMessageType::Exception, incoming_sequence_number);
            o_prot.write_message_begin(&message_ident)?;
            thrift::Error::write_application_error_to_out_protocol(&ret_err, o_prot)?;
            o_prot.write_message_end()?;
            o_prot.flush()
          },
        }
      },
    }
  }
}

impl <H: CollectorSyncHandler> TProcessor for CollectorSyncProcessor<H> {
  fn process(&self, i_prot: &mut dyn TInputProtocol, o_prot: &mut dyn TOutputProtocol) -> thrift::Result<()> {
    let message_ident = i_prot.read_message_begin()?;
    let res = match &*message_ident.name {
      "submitBatches" => {
        self.process_submit_batches(message_ident.sequence_number, i_prot, o_prot)
      },
      method => {
        Err(
          thrift::Error::Application(
            ApplicationError::new(
              ApplicationErrorKind::UnknownMethod,
              format!("unknown method {}", method)
            )
          )
        )
      },
    };
    thrift::server::handle_process_result(&message_ident, res, o_prot)
  }
}

//
// CollectorSubmitBatchesArgs
//

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct CollectorSubmitBatchesArgs {
  batches: Vec<Batch>,
}

impl CollectorSubmitBatchesArgs {
  fn read_from_in_protocol(i_prot: &mut dyn TInputProtocol) -> thrift::Result<CollectorSubmitBatchesArgs> {
    i_prot.read_struct_begin()?;
    let mut f_1: Option<Vec<Batch>> = None;
    loop {
      let field_ident = i_prot.read_field_begin()?;
      if field_ident.field_type == TType::Stop {
        break;
      }
      let field_id = field_id(&field_ident)?;
      match field_id {
        1 => {
          let list_ident = i_prot.read_list_begin()?;
          let mut val: Vec<Batch> = Vec::with_capacity(list_ident.size as usize);
          for _ in 0..list_ident.size {
            let list_elem_6 = Batch::read_from_in_protocol(i_prot)?;
            val.push(list_elem_6);
          }
          i_prot.read_list_end()?;
          f_1 = Some(val);
        },
        _ => {
          i_prot.skip(field_ident.field_type)?;
        },
      };
      i_prot.read_field_end()?;
    }
    i_prot.read_struct_end()?;
    verify_required_field_exists("CollectorSubmitBatchesArgs.batches", &f_1)?;
    let ret = CollectorSubmitBatchesArgs {
      batches: f_1.expect("auto-generated code should have checked for presence of required fields"),
    };
    Ok(ret)
  }
  fn write_to_out_protocol(&self, o_prot: &mut dyn TOutputProtocol) -> thrift::Result<()> {
    let struct_ident = TStructIdentifier::new("submitBatches_args");
    o_prot.write_struct_begin(&struct_ident)?;
    o_prot.write_field_begin(&TFieldIdentifier::new("batches", TType::List, 1))?;
    o_prot.write_list_begin(&TListIdentifier::new(TType::Struct, self.batches.len() as i32))?;
    for e in &self.batches {
      e.write_to_out_protocol(o_prot)?;
      o_prot.write_list_end()?;
    }
    o_prot.write_field_end()?;
    o_prot.write_field_stop()?;
    o_prot.write_struct_end()
  }
}

//
// CollectorSubmitBatchesResult
//

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct CollectorSubmitBatchesResult {
  result_value: Option<Vec<BatchSubmitResponse>>,
}

impl CollectorSubmitBatchesResult {
  fn read_from_in_protocol(i_prot: &mut dyn TInputProtocol) -> thrift::Result<CollectorSubmitBatchesResult> {
    i_prot.read_struct_begin()?;
    let mut f_0: Option<Vec<BatchSubmitResponse>> = None;
    loop {
      let field_ident = i_prot.read_field_begin()?;
      if field_ident.field_type == TType::Stop {
        break;
      }
      let field_id = field_id(&field_ident)?;
      match field_id {
        0 => {
          let list_ident = i_prot.read_list_begin()?;
          let mut val: Vec<BatchSubmitResponse> = Vec::with_capacity(list_ident.size as usize);
          for _ in 0..list_ident.size {
            let list_elem_7 = BatchSubmitResponse::read_from_in_protocol(i_prot)?;
            val.push(list_elem_7);
          }
          i_prot.read_list_end()?;
          f_0 = Some(val);
        },
        _ => {
          i_prot.skip(field_ident.field_type)?;
        },
      };
      i_prot.read_field_end()?;
    }
    i_prot.read_struct_end()?;
    let ret = CollectorSubmitBatchesResult {
      result_value: f_0,
    };
    Ok(ret)
  }
  fn write_to_out_protocol(&self, o_prot: &mut dyn TOutputProtocol) -> thrift::Result<()> {
    let struct_ident = TStructIdentifier::new("CollectorSubmitBatchesResult");
    o_prot.write_struct_begin(&struct_ident)?;
    if let Some(ref fld_var) = self.result_value {
      o_prot.write_field_begin(&TFieldIdentifier::new("result_value", TType::List, 0))?;
      o_prot.write_list_begin(&TListIdentifier::new(TType::Struct, fld_var.len() as i32))?;
      for e in fld_var {
        e.write_to_out_protocol(o_prot)?;
        o_prot.write_list_end()?;
      }
      o_prot.write_field_end()?;
      ()
    } else {
      ()
    }
    o_prot.write_field_stop()?;
    o_prot.write_struct_end()
  }
  fn ok_or(self) -> thrift::Result<Vec<BatchSubmitResponse>> {
    if self.result_value.is_some() {
      Ok(self.result_value.unwrap())
    } else {
      Err(
        thrift::Error::Application(
          ApplicationError::new(
            ApplicationErrorKind::MissingResult,
            "no result received for CollectorSubmitBatches"
          )
        )
      )
    }
  }
}

