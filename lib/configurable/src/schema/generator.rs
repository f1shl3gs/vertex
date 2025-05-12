use super::{Map, SchemaObject};
use crate::Configurable;

/// Schema generator.
///
/// This is the main entrypoint for storing the defined schemas within a given root schema, and
/// referencing existing schema definitions.
#[derive(Debug)]
pub struct SchemaGenerator {
    /// A JSON pointer to the expected location of referenceable subschemas within the resulting root schema.
    ///
    /// Defaults to `"#/definitions/"`.
    definitions_path: &'static str,

    pub(crate) definitions: Map<&'static str, SchemaObject>,
}

impl Default for SchemaGenerator {
    fn default() -> Self {
        SchemaGenerator {
            definitions_path: "#/definitions/",
            definitions: Map::default(),
        }
    }
}

impl SchemaGenerator {
    /// Attempts to find the schema that the given `schema` is referencing.
    ///
    /// If the given `schema` has a [`$ref`](../schema/struct.SchemaObject.html#structfield.reference)
    /// property which refers to another schema in `self`'s schema definitions, the referenced
    /// schema will be returned.  Otherwise, returns `None`.
    pub fn dereference(&self, schema: &SchemaObject) -> Option<&SchemaObject> {
        match &schema.reference {
            None => None,
            Some(reference) => {
                let definitions_path = self.definitions_path;

                if let Some(name) = reference.strip_prefix(definitions_path) {
                    self.definitions.get(name)
                } else {
                    None
                }
            }
        }
    }

    /// Generates a JSON Schema for the type `T`, and returns either the schema itself or a `$ref` schema referencing `T`'s schema.
    pub fn subschema_for<T: Configurable>(&mut self) -> SchemaObject {
        match T::reference() {
            Some(name) => {
                if !self.definitions.contains_key(name) {
                    let schema = T::generate_schema(self);
                    self.definitions.insert(name, schema);
                }

                SchemaObject::new_ref(format!("{}{}", self.definitions_path, name))
            }
            None => T::generate_schema(self),
        }
    }
}
