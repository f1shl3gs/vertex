use value::Value;

use crate::ByteSizeOf;

impl ByteSizeOf for Value {
    fn allocated_bytes(&self) -> usize {
        match self {
            Value::Bytes(b) => b.len(),
            Value::Object(map) => map.size_of(),
            Value::Array(arr) => arr.size_of(),
            _ => 0,
        }
    }
}
