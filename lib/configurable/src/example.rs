use std::cell::RefCell;

use crate::schema::generate_root_schema;
use crate::Configurable;
use schemars::schema::{InstanceType, RootSchema, Schema, SchemaObject, SingleOrVec};
use serde_json::Value;

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
        if let Some(reference) = self.root.schema.reference.as_ref() {
            if let Some(Schema::Object(root)) = self.get_referenced(reference) {
                self.visit_obj(root);
            }
        }

        let buf = self.buf.replace(Buf::new());
        buf.data
    }

    fn write_comment(&self, desc: Option<&String>) {
        if let Some(desc) = desc {
            let mut buf = self.buf.borrow_mut();
            buf.append_ident();
            buf.push('#');
            buf.push_str(desc.as_str());
            buf.push('\n');
        }
    }

    fn write_key(&self, s: &str) {
        let mut buf = self.buf.borrow_mut();
        buf.append_ident();
        buf.push_str(s);
        buf.push_str(": ");
    }

    fn visit_obj(&self, obj: &SchemaObject) {
        let obj = obj.object.as_ref().unwrap();

        for (k, v) in &obj.properties {
            let sub_obj = match v {
                Schema::Object(obj) => obj,
                _ => continue,
            };

            let desc = if let Some(meta) = sub_obj.metadata.as_ref() {
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
                        Some(Schema::Object(so)) => so,
                        _ => {
                            // TODO:
                            continue;
                        }
                    }
                }
                None => sub_obj,
            };

            self.write_comment(desc);
            self.write_key(k);

            if sub_obj.has_type(InstanceType::Object) {
                self.add_newline();
                self.incr_ident();
                self.visit_obj(sub_obj);
                self.decr_ident();

                continue;
            } else if sub_obj.has_type(InstanceType::Array) {
                self.visit_array(sub_obj);
            } else {
                self.visit_scalar(sub_obj);
            }

            self.add_newline();
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

        if item.has_type(InstanceType::Object) {
            self.visit_obj(item)
        } else if item.has_type(InstanceType::Array) {
            self.visit_array(item)
        } else {
            self.incr_ident();
            self.visit_scalar(item);
            self.decr_ident();
        };

        // serde_json::to_value(vec![value]).unwrap()
    }

    fn visit_scalar(&self, obj: &SchemaObject) {
        if let Some(value) = get_value(obj) {
            self.push_value(value);
            return;
        }

        if obj.has_type(InstanceType::Array) {
            self.push_value(&Value::Array(vec![]))
        } else if obj.has_type(InstanceType::String) {
            self.push_value(&Value::String("".to_string()));
        }
    }

    fn get_referenced(&self, key: &String) -> Option<&Schema> {
        if let Some(stripped) = key.strip_prefix("#/definitions/") {
            self.root.definitions.get(stripped)
        } else {
            self.root.definitions.get(key)
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
        buf.push('\n');
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

fn get_value(obj: &SchemaObject) -> Option<&Value> {
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
pub fn generate_example<T: Configurable + serde::Serialize>() -> String {
    let root_schema = generate_root_schema::<T>().expect("generate schema success");
    let visitor = Visitor::new(root_schema);
    visitor.example()
}
