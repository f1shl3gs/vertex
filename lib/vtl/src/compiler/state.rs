use value::{OwnedValuePath, Value};

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
            name: String::default(),
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
    pub fn apply_with_path(&mut self, kind: Kind, value_path: &OwnedValuePath) {
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
    /// Create a new variable if it is not exists. or return the index
    /// of exists variable.
    pub fn push(&mut self, name: &str) -> usize {
        match self
            .variables
            .iter()
            .rposition(|var| var.visible && var.name == name)
        {
            Some(index) => index,
            None => self.force_push(name.to_string()),
        }
    }

    /// Force push a variable, no matter it is exists already.
    pub fn force_push(&mut self, name: String) -> usize {
        self.variables.push(Variable {
            name,
            visible: true,
            value: Value::Null,
        });

        self.variables.len() - 1
    }

    #[inline]
    pub fn variable(&self, index: usize) -> &Variable {
        unsafe { self.variables.get_unchecked(index) }
    }

    #[inline]
    pub fn variable_mut(&mut self, index: usize) -> &mut Variable {
        unsafe { self.variables.get_unchecked_mut(index) }
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
}
