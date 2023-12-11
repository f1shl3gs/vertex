use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};

use value::path::PathPrefix;
use value::{OwnedTargetPath, Value};

pub enum Error {
    NotFound,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NotFound => f.write_str("not found"),
        }
    }
}

/// Any target object you want to remap using VTL has to implement this trait.
pub trait Target: Debug {
    /// Insert a given Value in the provided Target
    fn insert(&mut self, path: &OwnedTargetPath, value: Value) -> Result<(), Error>;

    /// Get a value for a given path, or Error::NotFound if no value is found.
    fn get(&mut self, path: &OwnedTargetPath) -> Result<&Value, Error>;

    /// Get a mutable reference to the value for a given path, or Error::NotFound if no
    /// value is found.
    fn get_mut(&mut self, path: &OwnedTargetPath) -> Result<&mut Value, Error>;

    /// Remove the given path from the target.
    ///
    /// Returns the removed value, if any. If compact is true, after deletion, if an empty
    /// object or array is left behind, it should be removed as well, cascading up to
    /// the root.
    fn remove(&mut self, path: &OwnedTargetPath, compact: bool) -> Result<Value, Error>;
}

#[derive(Clone, Debug)]
pub struct TargetValue {
    pub metadata: Value,
    pub value: Value,
}

impl Target for TargetValue {
    fn insert(&mut self, target_path: &OwnedTargetPath, value: Value) -> Result<(), Error> {
        let target = match target_path.prefix {
            PathPrefix::Event => &mut self.value,
            PathPrefix::Metadata => &mut self.metadata,
        };

        target.insert(&target_path.path, value);
        Ok(())
    }

    fn get(&mut self, target_path: &OwnedTargetPath) -> Result<&Value, Error> {
        let target = match target_path.prefix {
            PathPrefix::Event => &self.value,
            PathPrefix::Metadata => &self.metadata,
        };

        match target.get(&target_path.path) {
            Some(value) => Ok(value),
            None => Err(Error::NotFound),
        }
    }

    fn get_mut(&mut self, target_path: &OwnedTargetPath) -> Result<&mut Value, Error> {
        let target = match target_path.prefix {
            PathPrefix::Event => &mut self.value,
            PathPrefix::Metadata => &mut self.metadata,
        };

        match target.get_mut(&target_path.path) {
            Some(value) => Ok(value),
            None => Err(Error::NotFound),
        }
    }

    fn remove(&mut self, target_path: &OwnedTargetPath, compact: bool) -> Result<Value, Error> {
        let target = match target_path.prefix {
            PathPrefix::Event => &mut self.value,
            PathPrefix::Metadata => &mut self.metadata,
        };

        match target.remove(&target_path.path, compact) {
            Some(value) => Ok(value),
            None => Err(Error::NotFound),
        }
    }
}

pub struct Context<'a> {
    pub target: &'a mut dyn Target,
    pub variables: &'a mut HashMap<String, Value>,
}
