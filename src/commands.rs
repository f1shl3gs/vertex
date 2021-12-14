#![allow(clippy::print_stdout)] // tests

use clap::Parser;
use vertex::config::{
    ExtensionDescription, ProviderDescription, SinkDescription, SourceDescription,
    TransformDescription,
};

#[derive(Debug, Parser)]
pub enum Commands {
    Sources(Sources),
    Transforms(Transforms),
    Sinks(Sinks),
    Extensions(Extensions),
    Providers(Providers),
}

macro_rules! impl_list_and_example {
    ($typ:ident, $desc:ident) => {
        impl $typ {
            pub fn run(&self) {
                match &self.name {
                    Some(name) => match $desc::example(&name) {
                        Ok(example) => println!("{}", serde_yaml::to_string(&example).unwrap()),
                        Err(err) => {
                            println!("Generate example failed: {}", err);
                            std::process::exit(exitcode::UNAVAILABLE);
                        }
                    },

                    _ => {
                        for item in $desc::types() {
                            println!("{}", item);
                        }
                    }
                }
            }
        }
    };
}

#[derive(Debug, Parser)]
pub struct Sources {
    name: Option<String>,
}

impl_list_and_example!(Sources, SourceDescription);

#[derive(Debug, Parser)]
pub struct Transforms {
    name: Option<String>,
}

impl_list_and_example!(Transforms, TransformDescription);

#[derive(Debug, Parser)]
pub struct Sinks {
    name: Option<String>,
}

impl_list_and_example!(Sinks, SinkDescription);

#[derive(Debug, Parser)]
pub struct Extensions {
    name: Option<String>,
}

impl_list_and_example!(Extensions, ExtensionDescription);

#[derive(Debug, Parser)]
pub struct Providers {
    name: Option<String>,
}

impl Providers {
    pub fn run(&self) {
        match &self.name {
            Some(name) => match ProviderDescription::example(&name) {
                Ok(example) => println!("{}", serde_yaml::to_string(&example).unwrap()),
                Err(err) => {
                    println!("Generate example failed: {:?}", err);
                    std::process::exit(exitcode::UNAVAILABLE)
                }
            },
            _ => {
                for desc in inventory::iter::<ProviderDescription> {
                    println!("{}", desc.type_str)
                }
            }
        }
    }
}
