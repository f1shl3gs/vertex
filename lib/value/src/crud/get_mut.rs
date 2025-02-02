use crate::crud::ValueCollection;
use crate::path::BorrowedSegment;
use crate::Value;

pub fn get_mut<'a>(
    mut value: &mut Value,
    mut path_iter: impl Iterator<Item = BorrowedSegment<'a>>,
) -> Option<&mut Value> {
    loop {
        match (path_iter.next(), value) {
            (None, value) => return Some(value),
            (Some(BorrowedSegment::Field(key)), Value::Object(map)) => {
                match map.get_mut_value(key.as_ref()) {
                    None => return None,
                    Some(nested) => value = nested,
                }
            }
            (Some(BorrowedSegment::Index(index)), Value::Array(array)) => {
                match array.get_mut_value(&index) {
                    None => return None,
                    Some(nested) => value = nested,
                }
            }
            _ => return None,
        }
    }
}
