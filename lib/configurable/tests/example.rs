#![allow(clippy::print_stdout)]

use std::cell::RefCell;

use configurable::schema::generate_root_schema;
use configurable_derive::Configurable;
use schemars::schema::{InstanceType, RootSchema, Schema, SchemaObject, SingleOrVec};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[test]
fn generate_example() {
    #[derive(Configurable, Clone, Debug, Deserialize, Serialize)]
    struct Sub {
        offset: String,
    }

    fn default_timeout() -> u32 {
        11
    }

    #[derive(Clone, Debug, Configurable, Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    pub struct NtpConfig {
        /// Time for NTP round-trip. in seconds.
        ///
        /// blah.
        /// sss
        ///
        /// xxx
        #[serde(default = "default_timeout")]
        #[configurable(default = 10)]
        pub timeout: u32,

        /// Address for NTP client to connect
        #[configurable(format = "hostname", example = "pool.ntp.org")]
        pub pools: Vec<String>,

        sub: Sub,
    }

    let root_schema = generate_root_schema::<NtpConfig>().unwrap();

    let text = serde_json::to_string_pretty(&root_schema).unwrap();
    println!("{}", text);

    let visitor = Visitor::new(root_schema);
    let example = visitor.example();

    println!("{}", example)
}

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
}

struct Visitor {
    root: RootSchema,
    ident: u32,

    buf: RefCell<Buf>,
}

impl Visitor {
    pub fn new(root: RootSchema) -> Self {
        Self {
            root,
            ident: 0,
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

    fn append_ident(&self) {
        for _ in 0..self.ident {
            self.buf.borrow_mut().push(' ');
        }
    }

    fn write_comment(&self, obj: &SchemaObject) {
        if let Some(meta) = &obj.metadata {
            if let Some(desc) = &meta.description {
                self.append_ident();

                let mut buf = self.buf.borrow_mut();
                buf.push('#');
                buf.push_str(desc.as_str());
                buf.push('\n');
            }
        }
    }

    fn write_key(&self, s: &str) {
        let mut buf = self.buf.borrow_mut();

        buf.push_str(s);
        buf.push_str(": ");
    }

    fn visit_obj(&self, obj: &SchemaObject) {
        let obj = obj.object.as_ref().unwrap();

        for (k, v) in &obj.properties {
            let sub_schema = if v.is_ref() {
                if let Some(so) = self.get_referenced(k) {
                    so
                } else {
                    continue;
                }
            } else {
                v
            };

            let sub_obj = match sub_schema {
                Schema::Object(s) => s,
                _ => return,
            };

            if let Some(meta) = &sub_obj.metadata {
                if meta.deprecated {
                    continue;
                }
            }

            self.write_comment(sub_obj);
            self.write_key(k);

            if sub_obj.has_type(InstanceType::Object) {
                self.visit_obj(sub_obj);
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
            self.push_value(value)
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
