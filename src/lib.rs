pub mod config;
pub mod transforms;
pub mod sources;
pub mod topology;
pub mod trace;
pub mod signal;
pub mod duration;
mod shutdown;
mod sinks;
mod timezone;
mod pipeline;
mod buffers;
mod tls;
mod trigger;
mod app;
mod extensions;
mod error;
mod http;
mod template;
mod multiline;

pub use signal::{SignalHandler};

extern crate bloom;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate tracing;

#[macro_use]
extern crate internal;

pub(crate) use error::Error;

pub type Result<T> = std::result::Result<T, Error>;

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


pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

pub fn get_version() -> String {
    #[cfg(feature = "nightly")]
        let version = format!("{}-nightly", built_info::PKG_VERSION);

    #[cfg(not(feature = "nightly"))]
        let version = format!("{}-nightly", built_info::PKG_VERSION);

    version
}

pub fn hostname() -> std::io::Result<String> {
    Ok(::hostname::get()?.to_string_lossy().into())
}