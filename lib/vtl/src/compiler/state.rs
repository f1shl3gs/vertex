use value::path::{PathPrefix, TargetPath};
use value::{OwnedValuePath, Value};

use super::assignment::AssignmentTarget;
use super::Kind;
use super::ValueKind;

pub struct Variable {
    /// The name of this variable
    pub name: String,

    /// Once the variable leaves the block, it is not accessible anymore.
    ///
    /// ```text
    /// for key, value in map {
    ///     log("key/value:", key, value)
    /// }
    ///
    /// log("key", key) # undefined variable error
    /// ```
    pub visible: bool,

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
    pub fn kind(&self, path: &OwnedValuePath) -> Kind {
        if path.is_root() {
            return match &self.value {
                Value::Integer(i) => Kind::new(*i as u16),
                value => value.kind(),
            };
        }

        match self.value.get(path) {
            Some(Value::Integer(i)) => Kind::new(*i as u16),
            Some(value) => value.kind(),
            None => Kind::NULL,
        }
    }

    #[inline]
    fn apply_with_path(&mut self, kind: Kind, value_path: &OwnedValuePath) {
        self.value.insert(value_path, kind.inner());
    }

    #[inline]
    pub fn apply(&mut self, kind: Kind) {
        self.value = Value::Integer(kind.inner() as i64)
    }
}

#[derive(Default)]
pub struct TypeState {
    /// The key is variable name
    pub variables: Vec<Variable>,

    /// external environments
    pub target: Variable,
    pub metadata: Variable,
}

impl TypeState {
    /// Create a new variable if it is not exists.
    pub fn push(&mut self, name: &str) -> usize {
        match self
            .variables
            .iter()
            .rposition(|var| var.visible && var.name == name)
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
        }
    }

    pub fn variable(&self, index: usize) -> &Variable {
        unsafe { self.variables.get_unchecked(index) }
    }

    pub fn variable_mut(&mut self, index: usize) -> &mut Variable {
        unsafe { self.variables.get_unchecked_mut(index) }
    }

    pub fn apply(&mut self, target: &AssignmentTarget, kind: Kind) {
        match target {
            AssignmentTarget::Internal(index, path) => {
                let variable = unsafe { self.variables.get_unchecked_mut(*index) };
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
        assert_eq!(variable.kind(&value_target), Kind::BYTES);

        let value_target = parse_value_path("foo").unwrap();
        assert_eq!(variable.kind(&value_target), Kind::OBJECT);

        let value_target = parse_value_path("foo.foo").unwrap();
        assert_eq!(variable.kind(&value_target), Kind::NULL);
    }

    #[test]
    fn detail_array() {
        let mut variable = Variable::default();
        let value_target = parse_value_path("[1]").unwrap();

        variable.apply_with_path(Kind::BYTES, &value_target);
        assert_eq!(variable.kind(&value_target), Kind::BYTES);

        let value_target = parse_value_path("[0]").unwrap();
        assert_eq!(variable.kind(&value_target), Kind::NULL);
    }

    #[test]
    fn local() {
        let mut state = TypeState::default();
        state.push("foo"); // a dummy variable, to make suer index 0 did exists
        let target = AssignmentTarget::Internal(0, None);
        let kind = Kind::BYTES;

        state.apply(&target, kind);
    }
}
