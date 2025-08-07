use std::ops::Deref;

use serde::Serialize;
use serde_json::Value;

use super::{Map, Set};

/// The root object of a JSON Schema document.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RootSchema {
    /// The `$schema` keyword.
    ///
    /// See [JSON Schema 8.1.1. The "$schema" Keyword](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-8.1.1).
    #[serde(rename = "$schema")]
    pub meta_schema: &'static str,

    /// The root schema itself.
    #[serde(flatten)]
    pub schema: SchemaObject,

    /// The `definitions` keyword.
    ///
    /// In JSON Schema draft 2019-09 this was replaced by $defs, but in Schemars this is still
    /// serialized as `definitions` for backward-compatibility.
    ///
    /// See [JSON Schema 8.2.5. Schema Re-Use With "$defs"](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-8.2.5),
    /// and [JSON Schema (draft 07) 9. Schema Re-Use With "definitions"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-01#section-9).
    #[serde(alias = "$defs", skip_serializing_if = "Map::is_empty")]
    pub definitions: Map<&'static str, SchemaObject>,
}

/// A JSON Schema object.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SchemaObject {
    /// Properties which annotate the [`SchemaObject`] which typically have no effect when an object is being validated against the schema.
    #[serde(flatten)]
    pub metadata: Metadata,

    /// The `type` keyword.
    ///
    /// See [JSON Schema Validation 6.1.1. "type"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-6.1.1)
    /// and [JSON Schema 4.2.1. Instance Data Model](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-4.2.1).
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub instance_type: Option<SingleOrVec<InstanceType>>,

    /// The `format` keyword.
    ///
    /// See [JSON Schema Validation 7. A Vocabulary for Semantic Content With "format"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-7).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<&'static str>,

    /// The `$ref` keyword.
    ///
    /// See [JSON Schema 8.2.4.1. Direct References with "$ref"](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-8.2.4.1).
    #[serde(rename = "$ref", skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,

    /// The `const` keyword.
    ///
    /// See [JSON Schema Validation 6.1.3. "const"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-6.1.3)
    #[serde(rename = "const", skip_serializing_if = "Option::is_none")]
    pub const_value: Option<Value>,

    /// Properties of the [`SchemaObject`] which define validation assertions in terms of other schemas.
    #[serde(flatten)]
    pub subschemas: Option<Box<SubschemaValidation>>,

    /// Properties of the [`SchemaObject`] which define validation assertions for numbers.
    #[serde(flatten)]
    pub number: Option<Box<NumberValidation>>,

    /// Properties of the [`SchemaObject`] which define validation assertions for arrays.
    #[serde(flatten)]
    pub array: Option<Box<ArrayValidation>>,

    /// Properties of the [`SchemaObject`] which define validation assertions for objects.
    #[serde(flatten)]
    pub object: Option<Box<ObjectValidation>>,
}

impl SchemaObject {
    /// Creates a new `$ref` schema.
    ///
    /// The given reference string should be a URI reference. This will usually be a JSON Pointer
    /// in [URI Fragment representation](https://tools.ietf.org/html/rfc6901#section-6).
    pub fn new_ref(reference: String) -> Self {
        SchemaObject {
            reference: Some(reference),
            ..Default::default()
        }
    }

    pub fn new_object(description: Option<&'static str>) -> Self {
        SchemaObject {
            instance_type: Some(InstanceType::Object.into()),
            object: Some(Box::new(ObjectValidation {
                properties: Map::new(),
                required: Set::default(),
                ..Default::default()
            })),
            metadata: Metadata {
                description,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    pub fn one_of(subschemas: Vec<SchemaObject>, description: Option<&'static str>) -> Self {
        SchemaObject {
            metadata: Metadata {
                description,
                ..Default::default()
            },
            subschemas: Some(Box::new(SubschemaValidation {
                one_of: Some(subschemas),
                ..Default::default()
            })),
            ..Default::default()
        }
    }

    #[inline]
    pub fn const_value(value: &'static str) -> Self {
        SchemaObject {
            instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::String))),
            const_value: Some(Value::String(value.to_string())),
            ..Default::default()
        }
    }

    #[inline]
    pub fn null() -> Self {
        SchemaObject {
            instance_type: Some(InstanceType::Null.into()),
            ..Default::default()
        }
    }

    pub fn insert_property(
        &mut self,
        key: &'static str,
        required: bool,
        description: Option<&'static str>,
        mut subschema: SchemaObject,
    ) -> &mut Self {
        subschema.metadata.description = description;

        let object = self.object();
        object.properties.insert(key, subschema);
        if required {
            object.required.insert(key);
        }

        object.properties.get_mut(key).unwrap()
    }

    pub fn insert_tag(&mut self, key: &'static str, value: &'static str) {
        self.object().required.insert(key);

        self.object().properties.insert(
            key,
            SchemaObject {
                instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::String))),
                const_value: Some(Value::String(value.into())),
                ..Default::default()
            },
        );
    }

    pub fn insert_flatten(&mut self, mut subschema: SchemaObject) {
        if subschema.get_one_of().is_some() {
            if let Some(object) = &self.object {
                let all_of = if object.properties.is_empty() {
                    Some(vec![subschema])
                } else {
                    Some(vec![self.clone(), subschema])
                };

                self.subschemas = Some(Box::new(SubschemaValidation {
                    all_of,
                    ..Default::default()
                }));
            }

            self.object = None;
            self.instance_type = None;

            return;
        }

        if let Some(all_of) = self.get_all_of() {
            all_of.push(subschema);
            return;
        }

        // merge objects
        let to = self.object();
        for (k, v) in &subschema.object().properties {
            to.properties.insert(k, v.clone());
        }

        for required in &subschema.object().required {
            to.required.insert(required);
        }
    }

    pub fn set_default(&mut self, value: Value) {
        self.metadata.default = Some(value);
    }

    pub fn set_format(&mut self, format: &'static str) {
        self.format = Some(format);
    }

    pub fn add_example(&mut self, value: impl Into<Value>) {
        self.metadata.examples.push(value.into());
    }

    /// Returns `true` if `self` is a `$ref` schema.
    ///
    /// If `self` has `Some` [`reference`](struct.SchemaObject.html#structfield.reference) set, this returns `true`.
    /// Otherwise, returns `false`.
    pub fn is_ref(&self) -> bool {
        self.reference.is_some()
    }

    fn get_one_of(&mut self) -> Option<&mut Vec<SchemaObject>> {
        if let Some(subschemas) = &mut self.subschemas
            && let Some(one_of) = &mut subschemas.one_of
        {
            return Some(one_of);
        }

        None
    }

    fn get_all_of(&mut self) -> Option<&mut Vec<SchemaObject>> {
        if let Some(subschemas) = &mut self.subschemas
            && let Some(all_of) = &mut subschemas.all_of
        {
            return Some(all_of);
        }

        None
    }

    /// Returns `true` if `self` accepts values of the given type, according to the [`instance_type`] field.
    ///
    /// This is a basic check that always returns `true` if no `instance_type` is specified on the schema,
    /// and does not check any subschemas. Because of this, both `{}` and  `{"not": {}}` accept any type according
    /// to this method.
    pub fn has_type(&self, ty: InstanceType) -> bool {
        self.instance_type.as_ref().is_none_or(|x| x.contains(&ty))
    }

    /// Returns a mutable reference to this schema\'s [`ObjectValidation`](#structfield.object),
    /// creating it if it was `None`.
    pub fn object(&mut self) -> &mut ObjectValidation {
        self.object.get_or_insert_with(Default::default)
    }
}

/// Properties which annotate a [`SchemaObject`] which typically have no effect when an object is being validated against the schema.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Metadata {
    /// The `$id` keyword.
    ///
    /// See [JSON Schema 8.2.2. The "$id" Keyword](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-8.2.2).
    #[serde(rename = "$id", skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// The `description` keyword.
    ///
    /// See [JSON Schema Validation 9.1. "title" and "description"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-9.1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<&'static str>,

    /// The `default` keyword.
    ///
    /// See [JSON Schema Validation 9.2. "default"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-9.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,

    /// The `deprecated` keyword.
    ///
    /// See [JSON Schema Validation 9.3. "deprecated"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-9.3).
    #[serde(skip_serializing_if = "is_false")]
    pub deprecated: bool,

    /// The `examples` keyword.
    ///
    /// See [JSON Schema Validation 9.5. "examples"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-9.5).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<Value>,
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_false(b: &bool) -> bool {
    !b
}

/// Properties of a [`SchemaObject`] which define validation assertions in terms of other schemas.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SubschemaValidation {
    /// The `allOf` keyword.
    ///
    /// See [JSON Schema 9.2.1.1. "allOf"](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-9.2.1.1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all_of: Option<Vec<SchemaObject>>,

    /// The `anyOf` keyword.
    ///
    /// See [JSON Schema 9.2.1.2. "anyOf"](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-9.2.1.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any_of: Option<Vec<SchemaObject>>,

    /// The `oneOf` keyword.
    ///
    /// See [JSON Schema 9.2.1.3. "oneOf"](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-9.2.1.3).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub one_of: Option<Vec<SchemaObject>>,

    /// The `not` keyword.
    ///
    /// See [JSON Schema 9.2.1.4. "not"](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-9.2.1.4).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not: Option<Box<SchemaObject>>,
}

/// Properties of a [`SchemaObject`] which define validation assertions for numbers.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct NumberValidation {
    /// The `maximum` keyword.
    ///
    /// See [JSON Schema Validation 6.2.2. "maximum"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-6.2.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,

    /// The `minimum` keyword.
    ///
    /// See [JSON Schema Validation 6.2.4. "minimum"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-6.2.4).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
}

/// Properties of a [`SchemaObject`] which define validation assertions for arrays.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ArrayValidation {
    /// The `items` keyword.
    ///
    /// See [JSON Schema 9.3.1.1. "items"](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-9.3.1.1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<SingleOrVec<SchemaObject>>,
}

/// Properties of a [`SchemaObject`] which define validation assertions for objects.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ObjectValidation {
    /// The `properties` keyword.
    ///
    /// See [JSON Schema 9.3.2.1. "properties"](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-9.3.2.1).
    #[serde(skip_serializing_if = "Map::is_empty")]
    pub properties: Map<&'static str, SchemaObject>,

    /// The `required` keyword.
    ///
    /// See [JSON Schema Validation 6.5.3. "required"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-6.5.3).
    #[serde(skip_serializing_if = "Set::is_empty")]
    pub required: Set<&'static str>,

    /// The `additionalProperties` keyword.
    ///
    /// See [JSON Schema 9.3.2.3. "additionalProperties"](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-9.3.2.3).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_properties: Option<Box<SchemaObject>>,
}

/// The possible types of values in JSON Schema documents.
///
/// See [JSON Schema 4.2.1. Instance Data Model](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-4.2.1).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InstanceType {
    Null,
    Boolean,
    Object,
    Array,
    Number,
    String,
    Integer,
}

/// A type which can be serialized as a single item, or multiple items.
///
/// In some contexts, a `Single` may be semantically distinct from a `Vec` containing only item.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(untagged)]
pub enum SingleOrVec<T> {
    Single(Box<T>),
    Vec(Vec<T>),
}

impl<T> From<T> for SingleOrVec<T> {
    fn from(single: T) -> Self {
        SingleOrVec::Single(Box::new(single))
    }
}

impl<T> From<Vec<T>> for SingleOrVec<T> {
    fn from(vec: Vec<T>) -> Self {
        SingleOrVec::Vec(vec)
    }
}

impl<T: PartialEq> SingleOrVec<T> {
    /// Returns `true` if `self` is either a `Single` equal to `x`, or a `Vec` containing `x`.
    pub fn contains(&self, x: &T) -> bool {
        match self {
            SingleOrVec::Single(s) => s.deref() == x,
            SingleOrVec::Vec(v) => v.contains(x),
        }
    }
}
