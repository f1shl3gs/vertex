use std::cell::RefCell;

use schemars::schema::{
    InstanceType, ObjectValidation, RootSchema, Schema, SchemaObject, SingleOrVec,
};
use serde_json::{Map, Number, Value};

use crate::schema::generate_root_schema;
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
}

pub struct Visitor {
    root: RootSchema,
    buf: RefCell<Buf>,
}

impl Visitor {
    pub fn new(root: RootSchema) -> Self {
        Self {
            root,
            buf: Buf::new().into(),
        }
    }

    pub fn example(self) -> String {
        let root = if let Some(reference) = self.root.schema.reference.as_ref() {
            if let Some(Schema::Object(root)) = self.get_referenced(reference) {
                root
            } else {
                panic!("root schema is not found")
            }
        } else {
            &self.root.schema
        };

        // root must be a struct or an enum
        if let Some(subschemas) = &root.subschemas {
            if let Some(oneof) = &subschemas.one_of {
                if let Some(Schema::Object(obj)) = oneof.first() {
                    self.visit_obj(obj)
                } else {
                    panic!("one_of's first schema should be a SchemaObject")
                }
            } else if let Some(allof) = &subschemas.all_of {
                for schema in allof {
                    if let Schema::Object(obj) = schema {
                        self.visit_obj(obj)
                    }
                }
            }
        } else {
            self.visit_obj(root)
        }

        let buf = self.buf.replace(Buf::new());
        buf.data
    }

    fn write_comment(&self, desc: Option<&String>, required: bool) {
        if let Some(desc) = desc {
            let mut buf = self.buf.borrow_mut();
            buf.append_ident();
            buf.push('#');
            buf.push_str(desc.as_str());
            buf.push('\n');

            buf.append_ident();
            buf.push_str("#\n");
            buf.append_ident();
            if required {
                buf.push_str("# Required\n");
            } else {
                buf.push_str("# Optional\n");
            }
        }
    }

    fn write_key(&self, s: &str) {
        let mut buf = self.buf.borrow_mut();
        buf.append_ident();
        buf.push_str(s);
        buf.push_str(": ");
    }

    fn visit_obj(&self, obj: &SchemaObject) {
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
                    self.visit_obj(obj)
                }
            });

            return;
        }

        let obj = self.extract(obj);

        if obj.properties.is_empty() {
            self.push_value(&Value::Object(Map::new()));
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

                meta.description.as_ref()
            } else {
                None
            };

            let sub_obj = match &sub_obj.reference {
                Some(reference) => {
                    match self.get_referenced(reference) {
                        Some(Schema::Object(so)) => {
                            if let Some(meta) = so.metadata.as_ref() {
                                if desc.is_none() {
                                    desc = meta.description.as_ref();
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

            self.add_newline();
            self.write_comment(desc, required.contains(k));
            self.write_key(k);
            self.visit_schema_object(sub_obj);
        }
    }

    fn visit_array(&self, obj: &SchemaObject) {
        if let Some(meta) = obj.metadata.as_ref() {
            if let Some(example) = meta.examples.first() {
                // key already written
                self.add_newline();
                self.push_array_item(example);

                return;
            }

            self.push_value(&Value::Array(vec![]));

            return;
        }

        let arr = obj.array.as_ref().unwrap();

        let item = match arr.items.as_ref().unwrap() {
            SingleOrVec::Single(s) => match (*s).as_ref() {
                Schema::Object(ref sm) => sm,
                _ => return,
            },

            SingleOrVec::Vec(v) => match v.get(0).unwrap() {
                Schema::Object(so) => so,
                _ => return,
            },
        };

        self.visit_schema_object(item);
    }

    fn visit_scalar(&self, obj: &SchemaObject) {
        if let Some(value) = get_default_or_example(obj) {
            self.push_value(value);
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

        self.push_value(&value);
    }

    fn visit_schema_object(&self, obj: &SchemaObject) {
        if obj.has_type(InstanceType::Object) {
            // enum schema
            if let Some(subschema) = &obj.subschemas {
                if let Some(oneof) = &subschema.one_of {
                    // always pick first one
                    if let Some(Schema::Object(first)) = oneof.first() {
                        self.visit_schema_object(first);
                    }
                } else if let Some(allof) = &subschema.all_of {
                    self.incr_ident();
                    for schema in allof {
                        if let Schema::Object(obj) = schema {
                            self.visit_obj(obj)
                        }
                    }
                    self.decr_ident();

                    return;
                }
            } else if let Some(value) = &obj.const_value {
                self.push_value(value)
            } else {
                self.incr_ident();
                self.visit_obj(obj);
                self.decr_ident();
            }
        } else if obj.has_type(InstanceType::Array) {
            self.visit_array(obj)
        } else {
            self.visit_scalar(obj)
        }
    }

    fn get_referenced(&self, key: &String) -> Option<&Schema> {
        if let Some(stripped) = key.strip_prefix("#/definitions/") {
            self.root.definitions.get(stripped)
        } else {
            self.root.definitions.get(key)
        }
    }

    fn extract<'a>(&'a self, obj: &'a SchemaObject) -> &ObjectValidation {
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

    // buf
    fn push_value(&self, value: &Value) {
        let mut buf = self.buf.borrow_mut();
        let s = serde_json::to_string(value).unwrap();
        buf.push_str(&s);
        buf.push_str("\n");
    }

    fn push_array_item(&self, value: &Value) {
        let mut buf = self.buf.borrow_mut();
        let s = serde_json::to_string(value).unwrap();

        buf.push_str("- ");
        buf.push_str(&s);
    }

    fn incr_ident(&self) {
        let mut buf = self.buf.borrow_mut();
        buf.ident += 2;
    }

    fn decr_ident(&self) {
        let mut buf = self.buf.borrow_mut();
        buf.ident -= 2;
    }

    fn add_newline(&self) {
        let mut buf = self.buf.borrow_mut();
        buf.push('\n');
    }
}

fn is_all_of(obj: &SchemaObject) -> Option<&Vec<Schema>> {
    match &obj.subschemas {
        Some(sub) => sub.all_of.as_ref(),
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
    let visitor = Visitor::new(root_schema);
    visitor.example()
}
