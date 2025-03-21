use super::{Map, RootSchema, Schema, SchemaObject, visit::Visitor};
use crate::Configurable;

/// Settings to customize how schemas are generated.
#[derive(Debug)]
pub struct SchemaSettings {
    /// A JSON pointer to the expected location of referenceable subschemas within the resulting root schema.
    ///
    /// Defaults to `"#/definitions/"`.
    definitions_path: String,

    /// The URI of the meta-schema describing the structure of the generated schemas.
    ///
    /// Defaults to `"http://json-schema.org/draft-07/schema#"`.
    meta_schema: String,

    /// A list of visitors that get applied to all generated root schemas.
    visitors: Vec<Box<dyn Visitor>>,
}

impl Default for SchemaSettings {
    fn default() -> SchemaSettings {
        SchemaSettings::new()
    }
}

impl SchemaSettings {
    /// Creates `SchemaSettings` that conform to [JSON Schema 2019-09][json_schema_2019_09].
    ///
    /// [json_schema_2019_09]: https://json-schema.org/specification-links.html#2019-09-formerly-known-as-draft-8
    pub fn new() -> SchemaSettings {
        SchemaSettings {
            definitions_path: "#/definitions/".to_owned(),
            meta_schema: "https://json-schema.org/draft/2019-09/schema".to_string(),
            visitors: Vec::default(),
        }
    }

    /// Gets the definitions path used by this generator.
    pub fn definitions_path(&self) -> &str {
        &self.definitions_path
    }

    /// Appends the given visitor to the list of [visitors](SchemaSettings::visitors) for these `SchemaSettings`.
    pub fn with_visitor(mut self, visitor: impl Visitor + 'static) -> Self {
        self.visitors.push(Box::new(visitor));
        self
    }

    /// Creates a new [`SchemaGenerator`] using these settings.
    pub fn into_generator(self) -> SchemaGenerator {
        SchemaGenerator::new(self)
    }
}

/// Schema generator.
///
/// This is the main entrypoint for storing the defined schemas within a given root schema, and
/// referencing existing schema definitions.
#[derive(Debug, Default)]
pub struct SchemaGenerator {
    settings: SchemaSettings,
    definitions: Map<&'static str, Schema>,
}

impl From<SchemaSettings> for SchemaGenerator {
    fn from(settings: SchemaSettings) -> Self {
        settings.into_generator()
    }
}

impl SchemaGenerator {
    /// Creates a new `SchemaGenerator` using the given settings.
    pub fn new(settings: SchemaSettings) -> SchemaGenerator {
        SchemaGenerator {
            settings,
            ..Default::default()
        }
    }

    /// Gets the [`SchemaSettings`] being used by this `SchemaGenerator`.
    pub fn settings(&self) -> &SchemaSettings {
        &self.settings
    }

    /// Borrows the collection of all [referenceable](JsonSchema::is_referenceable) schemas that
    /// have been generated.
    ///
    /// The keys of the returned `Map` are the [schema names](JsonSchema::schema_name), and the
    /// values are the schemas themselves.
    pub fn definitions(&self) -> &Map<&'static str, Schema> {
        &self.definitions
    }

    /// Mutably borrows the collection of all [referenceable](JsonSchema::is_referenceable) schemas
    /// that have been generated.
    ///
    /// The keys of the returned `Map` are the [schema names](JsonSchema::schema_name), and the
    /// values are the schemas themselves.
    pub fn definitions_mut(&mut self) -> &mut Map<&'static str, Schema> {
        &mut self.definitions
    }

    /// Attempts to find the schema that the given `schema` is referencing.
    ///
    /// If the given `schema` has a [`$ref`](../schema/struct.SchemaObject.html#structfield.reference)
    /// property which refers to another schema in `self`'s schema definitions, the referenced
    /// schema will be returned.  Otherwise, returns `None`.
    pub fn dereference(&self, schema: &Schema) -> Option<&Schema> {
        match schema {
            Schema::Object(SchemaObject {
                reference: Some(schema_ref),
                ..
            }) => {
                let definitions_path = &self.settings().definitions_path;
                if schema_ref.starts_with(definitions_path) {
                    let name = &schema_ref[definitions_path.len()..];
                    self.definitions.get(name)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Converts this generator into a root schema, using the given `root_schema` as the top-level
    /// definition.
    ///
    /// This assumes the root schema was generated using this generator, such that any schema
    /// definitions referenced by `root_schema` refer to this generator.
    ///
    /// All other relevant settings (i.e. meta-schema) are carried over.
    pub fn into_root_schema(self, root_schema: SchemaObject) -> RootSchema {
        RootSchema {
            meta_schema: Some(self.settings.meta_schema),
            schema: root_schema,
            definitions: self.definitions,
        }
    }

    /// Generates a JSON Schema for the type `T`, and returns either the schema itself or a `$ref` schema referencing `T`'s schema.
    pub fn subschema_for<T: Configurable>(&mut self) -> SchemaObject {
        match T::reference() {
            Some(name) => {
                if !self.definitions().contains_key(name) {
                    self.definitions_mut().insert(name, Schema::Bool(false));

                    let schema = T::generate_schema(self);

                    self.definitions_mut().insert(name, Schema::Object(schema));
                }

                SchemaObject::new_ref(format!("{}{}", self.settings().definitions_path(), name))
            }
            None => T::generate_schema(self),
        }
    }
}
