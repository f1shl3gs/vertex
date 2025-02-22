use serde_json::Value;

use crate::Configurable;
use crate::schema::{
    InstanceType, ObjectValidation, RootSchema, Schema, SchemaObject, generate_root_schema,
};

struct Buf {
    ident: u32,
    data: String,
}

impl Buf {
    fn new() -> Self {
        Self {
            ident: 0,
            data: String::new(),
        }
    }

    #[inline]
    fn push(&mut self, c: char) {
        self.data.push(c)
    }

    #[inline]
    fn push_str(&mut self, s: &str) {
        self.data.push_str(s)
    }

    fn append_ident(&mut self) {
        for _ in 0..self.ident {
            self.data.push(' ');
        }
    }

    fn push_value(&mut self, value: &Value) {
        match value {
            Value::String(s) => {
                if s.is_empty() {
                    self.push_str(r#""""#)
                } else {
                    self.push_str(s)
                }
            }
            Value::Number(n) => {
                self.push_str(n.to_string().as_str());
            }
            Value::Bool(b) => {
                if *b {
                    self.push_str("true");
                } else {
                    self.push_str("false");
                }
            }
            Value::Null => self.push_str("null"),
            _ => {
                let s = serde_json::to_string(value).unwrap();
                self.push_str(&s);
            }
        }
    }

    fn incr_ident(&mut self) {
        self.ident += 2;
    }

    fn decr_ident(&mut self) {
        self.ident -= 2;
    }

    fn write_array(&mut self, obj: &SchemaObject) {
        if let Some(meta) = obj.metadata.as_ref() {
            if let Some(example) = meta.examples.first() {
                // key already written
                self.push_str("\n- ");
                self.push_value(example);

                return;
            }

            self.push_str("[]\n");

            return;
        }

        self.push_str("[]\n");
    }

    fn write_scalar(&mut self, obj: &SchemaObject) {
        if let Some(value) = get_default_or_example(obj) {
            self.push_value(value);
            self.push_str("\n");
            return;
        }

        if obj.has_type(InstanceType::Array) {
            self.push_str("[]\n")
        } else if obj.has_type(InstanceType::Null) {
            self.push_str("null\n")
        } else if obj.has_type(InstanceType::Boolean) {
            self.push_str("false\n")
        } else if obj.has_type(InstanceType::Number) {
            self.push_str("1.0\n");
        } else if obj.has_type(InstanceType::String) {
            self.push_str(r#""""#)
        } else if obj.has_type(InstanceType::Integer) {
            self.push_str("1\n")
        } else {
            self.push_str("null\n");
        }
    }

    fn write_comment(&mut self, desc: Option<&str>, required: bool) {
        if let Some(desc) = desc {
            desc.lines().for_each(|line| {
                self.append_ident();
                self.push_str("#");
                self.push_str(line);
                self.push('\n');
            });

            self.append_ident();
            self.push_str("#\n");
            self.append_ident();
            if required {
                self.push_str("# Required\n");
            } else {
                self.push_str("# Optional\n");
            }
        }
    }
}

struct Examplar {
    root: RootSchema,
}

impl Examplar {
    fn new(root: RootSchema) -> Self {
        Self { root }
    }

    fn generate(self) -> String {
        let root = if let Some(reference) = self.root.schema.reference.as_ref() {
            if let Some(Schema::Object(root)) = self.get_referenced(reference) {
                root
            } else {
                panic!("root schema is not found")
            }
        } else {
            &self.root.schema
        };

        let mut buf = Buf::new();
        // write comment for root
        if let Some(metadata) = &root.metadata {
            if let Some(desc) = &metadata.description {
                desc.lines().for_each(|line| {
                    buf.push('#');
                    buf.push_str(line);
                    buf.push('\n');
                })
            }
        }

        // root must be a struct or an enum
        if let Some(subschemas) = &root.subschemas {
            if let Some(oneof) = &subschemas.one_of {
                if let Some(Schema::Object(obj)) = oneof.first() {
                    self.visit_obj(&mut buf, obj)
                } else {
                    panic!("one_of's first schema should be a SchemaObject")
                }
            } else if let Some(allof) = &subschemas.all_of {
                for schema in allof {
                    if let Schema::Object(obj) = schema {
                        self.visit_obj(&mut buf, obj)
                    }
                }
            }
        } else {
            self.visit_obj(&mut buf, root)
        }

        buf.data
    }

    fn visit_obj(&self, buf: &mut Buf, obj: &SchemaObject) {
        let obj = match &obj.reference {
            Some(reference) => {
                if let Some(Schema::Object(obj)) = self.get_referenced(reference) {
                    obj
                } else {
                    return;
                }
            }
            None => obj,
        };

        if let Some(all_of) = is_all_of(obj) {
            all_of.iter().for_each(|schema| {
                if let Schema::Object(obj) = schema {
                    self.visit_obj(buf, obj)
                }
            });

            return;
        }

        if let Some(one_of) = is_one_of(obj) {
            // always choose the first one
            if let Schema::Object(first) = one_of.first().expect("oneOf should not be empty") {
                self.visit_obj(buf, first);
            } else {
                panic!("expect object schema")
            }

            return;
        }

        let obj = self.extract(obj);

        if obj.properties.is_empty() {
            buf.push_str("{}\n");
            return;
        }

        let required = &obj.required;
        for (k, v) in &obj.properties {
            let sub_obj = match v {
                Schema::Object(obj) => obj,
                _ => continue,
            };

            let mut desc = if let Some(meta) = sub_obj.metadata.as_ref() {
                if meta.deprecated {
                    continue;
                }

                meta.description
            } else {
                None
            };

            let sub_obj = match &sub_obj.reference {
                Some(reference) => {
                    match self.get_referenced(reference) {
                        Some(Schema::Object(so)) => {
                            if let Some(meta) = so.metadata.as_ref() {
                                if desc.is_none() {
                                    desc = meta.description;
                                }
                            }

                            so
                        }
                        _ => {
                            // TODO:
                            continue;
                        }
                    }
                }
                None => sub_obj,
            };

            buf.push('\n');
            buf.write_comment(desc, required.contains(k));

            // write key field
            buf.append_ident();
            buf.push_str(k);
            buf.push_str(": ");

            // write value
            self.visit_schema_object(buf, sub_obj);
        }
    }

    fn visit_schema_object(&self, buf: &mut Buf, obj: &SchemaObject) {
        if obj.has_type(InstanceType::Object) {
            // enum schema
            if let Some(subschema) = &obj.subschemas {
                if let Some(oneof) = &subschema.one_of {
                    // always pick first one
                    if let Some(Schema::Object(first)) = oneof.first() {
                        self.visit_schema_object(buf, first);
                    }
                } else if let Some(allof) = &subschema.all_of {
                    buf.incr_ident();
                    for schema in allof {
                        if let Schema::Object(obj) = schema {
                            self.visit_obj(buf, obj)
                        }
                    }
                    buf.decr_ident();

                    return;
                }
            } else if let Some(value) = &obj.const_value {
                buf.push_value(value);
                buf.push_str("\n");
            } else {
                buf.incr_ident();
                self.visit_obj(buf, obj);
                buf.decr_ident();
            }
        } else if obj.has_type(InstanceType::Array) {
            buf.write_array(obj)
        } else {
            buf.write_scalar(obj)
        }
    }

    fn get_referenced(&self, key: &str) -> Option<&Schema> {
        if let Some(stripped) = key.strip_prefix("#/definitions/") {
            self.root.definitions.get(stripped)
        } else {
            self.root.definitions.get(key)
        }
    }

    fn extract<'a>(&'a self, obj: &'a SchemaObject) -> &'a ObjectValidation {
        match &obj.object {
            Some(obj) => obj,
            None => {
                // flatten field with enum type goes here
                if let Some(subschemas) = &obj.subschemas {
                    if let Some(oneof) = &subschemas.one_of {
                        // handle first only
                        if let Some(Schema::Object(first)) = oneof.first() {
                            return first
                                .object
                                .as_ref()
                                .expect("flattened field cannot be empty");
                        }
                    }

                    panic!("subschemas should have a non-empty one_of");
                } else {
                    panic!("schema object should have at least one of `object` or `subschemas`");
                }
            }
        }
    }
}

fn is_all_of(obj: &SchemaObject) -> Option<&Vec<Schema>> {
    match &obj.subschemas {
        Some(sub) => sub.all_of.as_ref(),
        None => None,
    }
}

fn is_one_of(obj: &SchemaObject) -> Option<&Vec<Schema>> {
    match &obj.subschemas {
        Some(sub) => sub.one_of.as_ref(),
        None => None,
    }
}

fn get_default_or_example(obj: &SchemaObject) -> Option<&Value> {
    if let Some(meta) = obj.metadata.as_ref() {
        if meta.deprecated {
            return None;
        }

        if meta.default.is_some() {
            return meta.default.as_ref();
        }

        return meta.examples.first();
    }

    None
}

/// Generate YAML example from a JSON Schema
pub fn generate_config<T: Configurable>() -> String {
    let root_schema = generate_root_schema::<T>();
    Examplar::new(root_schema).generate()
}

#[inline]
pub fn generate_config_with_schema(schema: RootSchema) -> String {
    Examplar::new(schema).generate()
}
