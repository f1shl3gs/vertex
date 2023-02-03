#![allow(clippy::print_stdout)]

use configurable::schema::generate_root_schema;
use configurable_derive::Configurable;
use indexmap::IndexMap;
use schemars::schema::{
    ArrayValidation, InstanceType, ObjectValidation, RootSchema, Schema, SchemaObject, SingleOrVec,
};
use serde::{Deserialize, Serialize};

#[test]
fn generate_example() {
    #[derive(Configurable, Clone, Debug, Deserialize, Serialize)]
    struct Sub {
        offset: String,
    }

    #[derive(Clone, Debug, Configurable, Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    pub struct NtpConfig {
        /// Time for NTP round-trip. in seconds.
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

    let text = serde_json::to_string_pretty(&example).unwrap();
    println!("{}", text)
}

struct Visitor {
    root: RootSchema,
}

impl Visitor {
    pub fn new(root: RootSchema) -> Self {
        Self { root }
    }

    pub fn example(&self) -> serde_json::Value {
        match &self.root.schema.reference {
            Some(reference) => {
                if let Some(Schema::Object(root)) = self.get_referenced(reference) {
                    let obj = &root.object.as_ref().unwrap();
                    return self.visit_obj(obj);
                }
            }
            None => {}
        }

        serde_json::Value::Null
    }

    fn visit_array(&self, arr: &ArrayValidation) -> serde_json::Value {
        let item = match arr.items.as_ref().unwrap() {
            SingleOrVec::Single(s) => match (*s).as_ref() {
                Schema::Object(ref sm) => sm,
                _ => return serde_json::Value::Null,
            },

            SingleOrVec::Vec(v) => match v.get(0).unwrap() {
                Schema::Object(so) => so,
                _ => return serde_json::Value::Null,
            },
        };

        let value = if item.has_type(InstanceType::Object) {
            self.visit_obj(item.object.as_ref().unwrap())
        } else if item.has_type(InstanceType::Array) {
            self.visit_array(item.array.as_ref().unwrap())
        } else {
            match self.visit_scalar(item) {
                serde_json::Value::Null => {
                    return serde_json::to_value(Vec::<serde_json::Value>::new()).unwrap()
                }
                v => v,
            }
        };

        serde_json::to_value(vec![value]).unwrap()
    }

    fn visit_scalar(&self, obj: &SchemaObject) -> serde_json::Value {
        match obj.metadata.as_ref() {
            Some(meta) => {
                if meta.deprecated {
                    return serde_json::Value::Null;
                }

                if let Some(d) = &meta.default {
                    return d.clone();
                }

                serde_json::to_value(meta.examples.clone()).unwrap()
            }
            None => serde_json::Value::Null,
        }
    }

    fn visit_obj(&self, obj: &ObjectValidation) -> serde_json::Value {
        let mut map = IndexMap::new();

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
                _ => return serde_json::Value::Null,
            };

            if let Some(meta) = &sub_obj.metadata {
                if meta.deprecated {
                    continue;
                }

                if !meta.examples.is_empty() {
                    map.insert(k, serde_json::to_value(meta.examples.clone()).unwrap());
                    continue;
                }
            }

            if sub_obj.has_type(InstanceType::Object) {
                let mo = sub_obj.object.as_ref().unwrap();
                map.insert(k, self.visit_obj(mo));
            } else if sub_obj.has_type(InstanceType::Array) {
                let arr = sub_obj.array.as_ref().unwrap();
                map.insert(k, self.visit_array(arr));
            } else {
                map.insert(k, self.visit_scalar(sub_obj));
            }
        }

        serde_json::to_value(map).unwrap()
    }

    fn get_referenced(&self, key: &String) -> Option<&Schema> {
        if let Some(stripped) = key.strip_prefix("#/definitions/") {
            self.root.definitions.get(stripped)
        } else {
            self.root.definitions.get(key)
        }
    }
}
