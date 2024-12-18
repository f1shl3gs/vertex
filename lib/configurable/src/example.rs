use serde_json::{Map, Number, Value};

use crate::schema::{
    generate_root_schema, InstanceType, ObjectValidation, RootSchema, Schema, SchemaObject,
};
use crate::Configurable;

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
    fn push(&mut self, c: char) {
        self.data.push(c)
    }

    fn push_str(&mut self, s: &str) {
        self.data.push_str(s)
    }

    fn append_ident(&mut self) {
        for _ in 0..self.ident {
            self.data.push(' ');
        }
    }

    fn incr_ident(&mut self) {
        self.ident += 2;
    }

    fn decr_ident(&mut self) {
        self.ident -= 2;
    }

    fn push_value(&mut self, value: &Value) {
        match value {
            Value::String(s) => {
                if s.is_empty() {
                    self.push_str(r#""""#);
                } else {
                    self.push_str(s);
                }
            }
            Value::Number(n) => {
                self.push_str(&n.to_string());
            }
            Value::Bool(b) => {
                if *b {
                    self.push_str("true");
                } else {
                    self.push_str("false");
                }
            }
            Value::Null => {
                self.push_str("null");
            }
            _ => {
                let s = serde_json::to_string(value).unwrap();
                self.push_str(&s);
            }
        }

        self.push_str("\n");
    }

    fn write_comment(&mut self, desc: Option<&str>, required: bool) {
        if let Some(desc) = desc {
            for line in desc.lines() {
                self.append_ident();
                self.push_str("#");
                self.push_str(line);
                self.push('\n');
            }

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

    fn push_array_item(&mut self, value: &Value) {
        let s = serde_json::to_string(value).unwrap();

        self.push_str("- ");
        self.push_str(&s);
    }
}

pub struct Examplar {
    root: RootSchema,
}

impl Examplar {
    pub fn new(root: RootSchema) -> Self {
        Self { root }
    }

    pub fn generate(self) -> String {
        let root = if let Some(reference) = &self.root.schema.reference {
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
                for line in desc.lines() {
                    buf.push('#');
                    buf.push_str(line);
                    buf.push('\n');
                }
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
            buf.push_value(&Value::Object(Map::new()));
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
            buf.append_ident();
            buf.push_str(k);
            buf.push_str(": ");

            self.visit_schema_object(buf, sub_obj);
        }
    }

    fn visit_array(&self, buf: &mut Buf, obj: &SchemaObject) {
        if let Some(meta) = obj.metadata.as_ref() {
            if let Some(example) = meta.examples.first() {
                // key already written
                buf.push('\n');
                buf.push_array_item(example);

                return;
            }

            buf.push_value(&Value::Array(vec![]));

            return;
        }

        buf.push_value(&Value::Array(vec![]));
    }

    fn visit_scalar(&self, buf: &mut Buf, obj: &SchemaObject) {
        if let Some(value) = get_default_or_example(obj) {
            buf.push_value(value);
            return;
        }

        let value = if obj.has_type(InstanceType::Array) {
            Value::Array(vec![])
        } else if obj.has_type(InstanceType::Null) {
            Value::Null
        } else if obj.has_type(InstanceType::Boolean) {
            Value::Bool(false)
        } else if obj.has_type(InstanceType::Number) {
            Value::Number(Number::from_f64(1.0f64).unwrap())
        } else if obj.has_type(InstanceType::String) {
            Value::String("".to_owned())
        } else if obj.has_type(InstanceType::Integer) {
            Value::Number(1.into())
        } else {
            Value::Null
        };

        buf.push_value(&value);
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
                buf.push_value(value)
            } else {
                buf.incr_ident();
                self.visit_obj(buf, obj);
                buf.decr_ident();
            }
        } else if obj.has_type(InstanceType::Array) {
            self.visit_array(buf, obj)
        } else {
            self.visit_scalar(buf, obj)
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
    let root_schema = generate_root_schema::<T>().expect("generate schema success");
    let visitor = Examplar::new(root_schema);
    visitor.generate()
}
