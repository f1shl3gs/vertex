use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};

use serde::Serialize;
use value::path::{PathPrefix, TargetPath};
use value::{OwnedTargetPath, Value};

pub enum Error {
    NotFound,

    InvalidPath { expected: &'static str },

    InvalidValue { expected: &'static str },
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NotFound => f.write_str("not found"),
            Error::InvalidPath { expected } => {
                f.write_str("expected one of ")?;
                f.write_str(expected)
            }
            Error::InvalidValue { expected } => {
                f.write_str("expected one of ")?;
                f.write_str(expected)
            }
        }
    }
}

/// Any target object you want to remap using VTL has to implement this trait.
pub trait Target: Debug {
    /// Insert a given Value in the provided Target
    fn insert(&mut self, path: &OwnedTargetPath, value: Value) -> Result<(), Error>;

    /// Get a value for a given path, or Error::NotFound if no value is found.
    fn get(&mut self, path: &OwnedTargetPath) -> Result<Option<&Value>, Error>;

    /// Get a mutable reference to the value for a given path, or Error::NotFound if no
    /// value is found.
    fn get_mut(&mut self, path: &OwnedTargetPath) -> Result<Option<&mut Value>, Error>;

    /// Remove the given path from the target.
    ///
    /// Returns the removed value, if any. If compact is true, after deletion, if an empty
    /// object or array is left behind, it should be removed as well, cascading up to
    /// the root.
    fn remove(&mut self, path: &OwnedTargetPath, compact: bool) -> Result<Option<Value>, Error>;
}

#[derive(Clone, Debug, Serialize)]
pub struct TargetValue {
    pub metadata: Value,
    pub value: Value,
}

impl Target for TargetValue {
    fn insert(&mut self, target_path: &OwnedTargetPath, value: Value) -> Result<(), Error> {
        let target = match target_path.prefix() {
            PathPrefix::Event => &mut self.value,
            PathPrefix::Metadata => &mut self.metadata,
        };

        target.insert(target_path.value_path(), value);
        Ok(())
    }

    fn get(&mut self, target_path: &OwnedTargetPath) -> Result<Option<&Value>, Error> {
        let target = match target_path.prefix() {
            PathPrefix::Event => &self.value,
            PathPrefix::Metadata => &self.metadata,
        };

        Ok(target.get(target_path.value_path()))
    }

    fn get_mut(&mut self, target_path: &OwnedTargetPath) -> Result<Option<&mut Value>, Error> {
        let target = match target_path.prefix() {
            PathPrefix::Event => &mut self.value,
            PathPrefix::Metadata => &mut self.metadata,
        };

        Ok(target.get_mut(target_path.value_path()))
    }

    fn remove(
        &mut self,
        target_path: &OwnedTargetPath,
        compact: bool,
    ) -> Result<Option<Value>, Error> {
        let target = match target_path.prefix() {
            PathPrefix::Event => &mut self.value,
            PathPrefix::Metadata => &mut self.metadata,
        };

        Ok(target.remove(target_path.value_path(), compact))
    }
}

pub struct Context<'a> {
    pub target: &'a mut dyn Target,
    pub variables: &'a mut HashMap<String, Value>,
}
