#![allow(dead_code)]

use std::slice::Iter;

use value::path::{PathPrefix, TargetPath};
use value::{OwnedValuePath, Value};

use super::assignment::AssignmentTarget;
use super::Kind;
use super::ValueKind;

/// The state used at runtime to track changes as they happen.
pub struct RuntimeState {
    /// The Value stored in each variable.
    variables: Vec<Value>,
    // TODO: add TimeZone support
    // timezone: Tz
}

impl RuntimeState {
    #[inline]
    pub fn new() -> Self {
        Self { variables: vec![] }
    }

    #[inline(always)]
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

pub struct Variable {
    /// The name of this variable
    name: String,

    /// Once the variable leaves the block, it is not accessible anymore.
    ///
    /// ```text
    /// for key, value in map {
    ///     log("key/value:", key, value)
    /// }
    ///
    /// log("key", key) # undefined variable error
    /// ```
    visible: bool,

    /// This field is not really the "VALUE", it is used to store the
    /// variable kind and the structure of an object.
    ///
    /// e.g.
    ///   - Value::Integer(Kind::Numeric)
    ///   - Value::Object({
    ///         "foo": Value::Integer(Kind::Integer)
    ///     })
    value: Value,
}

impl Default for Variable {
    fn default() -> Self {
        Variable {
            name: "".to_string(),
            visible: true,
            value: Value::Null,
        }
    }
}

impl Variable {
    pub fn kind(&self, path: Option<&OwnedValuePath>) -> Kind {
        match path {
            Some(path) => match self.value.get(path) {
                Some(Value::Integer(i)) => Kind::new(*i as u16),
                Some(value) => value.kind(),
                None => Kind::NULL,
            },
            None => match &self.value {
                Value::Integer(i) => Kind::new(*i as u16),
                value => value.kind(),
            },
        }
    }

    #[inline]
    fn apply_with_path(&mut self, kind: Kind, value_path: &OwnedValuePath) {
        self.value.insert(value_path, kind.inner());
    }

    #[inline]
    fn apply(&mut self, kind: Kind) {
        self.value = Value::Integer(kind.inner() as i64)
    }
}

#[derive(Default)]
pub struct TypeState {
    /// The key is variable name
    variables: Vec<Variable>,

    /// external environments
    pub target: Variable,
    pub metadata: Variable,
}

impl TypeState {
    #[inline]
    pub fn variables(&self) -> Iter<'_, Variable> {
        self.variables.iter()
    }

    pub fn variable(&self, ident: &str) -> Option<&Variable> {
        self.variables
            .iter()
            .rfind(|var| var.visible && var.name == ident)
    }

    pub fn apply(&mut self, target: &AssignmentTarget, kind: Kind) {
        match target {
            AssignmentTarget::Internal(name, path) => {
                let index = match self
                    .variables
                    .iter()
                    .rposition(|var| var.visible && &var.name == name)
                {
                    Some(index) => index,
                    None => {
                        self.variables.push(Variable {
                            name: name.to_string(),
                            visible: true,
                            value: Value::Null,
                        });

                        self.variables.len() - 1
                    }
                };

                let variable = unsafe { self.variables.get_unchecked_mut(index) };
                match path {
                    Some(path) => variable.apply_with_path(kind, path),
                    None => variable.apply(kind),
                }
            }
            AssignmentTarget::External(path) => {
                let value_path = path.value_path();

                match path.prefix() {
                    PathPrefix::Event => self.target.apply_with_path(kind, value_path),
                    PathPrefix::Metadata => self.metadata.apply_with_path(kind, value_path),
                }
            }
        }
    }

    pub fn apply_variable(&mut self, ident: &str, kind: Kind) {
        let variable = self
            .variables
            .iter_mut()
            .rfind(|var| var.visible && var.name == ident)
            .expect("must exists");

        variable.apply(kind);
    }

    pub fn get_variable_kind(&self, ident: &str) -> Kind {
        self.variables
            .iter()
            .rfind(|var| var.visible && var.name == ident)
            .expect("must exists")
            .kind(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use value::parse_value_path;

    #[test]
    fn detail_map() {
        let mut variable = Variable::default();
        let value_target = parse_value_path("foo.bar").unwrap();

        variable.apply_with_path(Kind::BYTES, &value_target);
        assert_eq!(variable.kind(Some(&value_target)), Kind::BYTES);

        let value_target = parse_value_path("foo").unwrap();
        assert_eq!(variable.kind(Some(&value_target)), Kind::OBJECT);

        let value_target = parse_value_path("foo.foo").unwrap();
        assert_eq!(variable.kind(Some(&value_target)), Kind::NULL);
    }

    #[test]
    fn detail_array() {
        let mut variable = Variable::default();
        let value_target = parse_value_path("[1]").unwrap();

        variable.apply_with_path(Kind::BYTES, &value_target);
        assert_eq!(variable.kind(Some(&value_target)), Kind::BYTES);

        let value_target = parse_value_path("[0]").unwrap();
        assert_eq!(variable.kind(Some(&value_target)), Kind::NULL);
    }

    #[test]
    fn local() {
        let mut state = TypeState::default();
        let target = AssignmentTarget::Internal("foo".to_string(), None);
        let kind = Kind::BYTES;

        state.apply(&target, kind);
    }
}
