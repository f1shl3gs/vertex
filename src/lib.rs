pub mod config;
pub mod transforms;
pub mod sources;
pub mod vertex_core;
pub mod topology;
mod event;
mod shutdown;
mod sinks;
mod timezone;
pub mod duration;
mod pipeline;
mod buffers;
pub mod signal;
mod tls;
mod trigger;

pub use signal::{SignalHandler};

extern crate bloom;

extern crate slog;
#[macro_use]
extern crate slog_scope;
extern crate slog_term;

pub use vertex_core::{Result, Error};

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    #[typetag::serde(tag = "type")]
    trait Plugin {
        fn name(&self) -> String;
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct Foo {
        name: String,
    }

    #[typetag::serde(name = "foo")]
    impl Plugin for Foo {
        fn name(&self) -> String {
            self.name.clone()
        }
    }

    #[derive(Deserialize, Serialize)]
    struct Bar {
        name: String,
    }

    #[typetag::serde(name = "bar")]
    impl Plugin for Bar {
        fn name(&self) -> String {
            self.name.clone()
        }
    }

    #[derive(Deserialize, Serialize)]
    struct Foo1 {
        name1: String,
    }

    #[typetag::serde(name = "foo")]
    impl Plugin for Foo1 {
        fn name(&self) -> String {
            self.name1.clone()
        }
    }

    #[derive(Deserialize)]
    struct Reg {
        pub p1: Vec<Box<dyn Plugin>>,
        pub p2: Vec<Box<dyn Plugin>>,
    }

    #[test]
    fn deserialize() {
        let text = "\
p1:
- type: foo
  name: p1
p2:
- type: bar
  name1: p2
";

        let reg: Reg = serde_yaml::from_str(text).unwrap();
        for plugin in reg.p1.iter() {
            println!("{:?}", plugin.name())
        }

        for plugin in reg.p2.iter() {
            println!("{:?}", plugin.name())
        }
    }
}