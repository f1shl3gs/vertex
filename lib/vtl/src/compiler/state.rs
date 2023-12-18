#![allow(dead_code)]

use std::collections::HashMap;

use value::path::{PathPrefix, TargetPath};
use value::{OwnedValuePath, Value};

use super::assignment::AssignmentTarget;
use super::query::Query;
use super::Kind;
use super::ValueKind;

/// The state used at runtime to track changes as they happen.
pub struct RuntimeState {
    /// The Value stored in each variable.
    variables: Vec<Value>,
}

impl RuntimeState {
    #[inline]
    pub fn new() -> Self {
        Self { variables: vec![] }
    }

    #[inline]
    pub fn get(&self, index: usize) -> &Value {
        unsafe {
            // SAFETY: index checked at compile-time
            self.variables.get_unchecked(index)
        }
    }

    #[inline]
    pub fn get_mut(&mut self, index: usize) -> &Value {
        unsafe {
            // SAFETY: index checked at compile-time
            self.variables.get_unchecked_mut(index)
        }
    }

    pub fn push(&mut self, v: Value) -> usize {
        self.variables.push(v);
        self.variables.len() - 1
    }
}

struct Details {
    /// It's always hard to handle complex type, like Kind::Numeric
    /// or Kind::Container.
    kind: Kind,

    /// The value is not really the "VALUE", it is used to store the
    /// structure.
    ///
    /// e.g. `foo.bar = "foo"` then the `foo` must be an object
    value: Value,
}

impl Default for Details {
    fn default() -> Self {
        Details {
            kind: Kind::UNDEFINED,
            value: Value::Null,
        }
    }
}

impl Details {
    fn set(&mut self, value_path: &OwnedValuePath, kind: Kind) {
        self.value.insert(value_path, kind.inner());
    }

    fn get(&self, value_path: &OwnedValuePath) -> Kind {
        match self.value.get(value_path) {
            Some(Value::Integer(i)) => Kind::new(*i as u16),
            Some(value) => value.kind(),
            None => Kind::UNDEFINED,
        }
    }
}

#[derive(Default)]
pub struct TypeState {
    /// The key is variable name
    local: HashMap<String, Details>,

    /// external environments
    target: Details,
    metadata: Details,
}

impl TypeState {
    pub fn apply(&mut self, target: &AssignmentTarget, kind: Kind) {
        match target {
            AssignmentTarget::Internal(name, path) => {
                let details = self.local.entry(name.to_string()).or_default();

                match path {
                    Some(value_path) => {
                        details.kind = Kind::OBJECT;
                        details.value.insert(value_path, kind.inner());
                    }
                    None => details.kind = kind,
                }
            }
            AssignmentTarget::External(path) => {
                let value_path = path.value_path();

                match path.prefix() {
                    PathPrefix::Event => self.target.set(value_path, kind),
                    PathPrefix::Metadata => self.metadata.set(value_path, kind),
                }
            }
        }
    }

    pub fn apply_variable(&mut self, ident: &str, kind: Kind) {
        let detail = self.local.entry(ident.to_string()).or_default();

        detail.kind = kind;
        detail.value = Value::Null;
    }

    pub fn get_variable_kind(&self, ident: &str) -> Kind {
        self.local
            .get(ident)
            .expect("variable is checked at compile time")
            .kind
    }

    pub fn get_query_kind(&self, query: &Query) -> Kind {
        match query {
            Query::External(target_path) => {
                let value_path = target_path.value_path();
                match target_path.prefix() {
                    PathPrefix::Event => self.target.get(value_path),
                    PathPrefix::Metadata => self.metadata.get(value_path),
                }
            }
            Query::Internal(ident, value_path) => {
                let detail = self
                    .local
                    .get(ident)
                    .expect("variable already checked at compile time");

                detail.get(value_path)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use value::parse_value_path;

    #[test]
    fn detail() {
        let mut detail = Details::default();
        let value_target = parse_value_path("foo.bar").unwrap();

        detail.set(&value_target, Kind::BYTES);
        assert_eq!(detail.get(&value_target), Kind::BYTES);
        let value_target = parse_value_path("foo").unwrap();
        assert_eq!(detail.get(&value_target), Kind::OBJECT);
    }

    #[test]
    fn local() {
        let mut state = TypeState::default();
        let target = AssignmentTarget::Internal("foo".to_string(), None);
        let kind = Kind::BYTES;

        state.apply(&target, kind);
    }
}
