mod validate;

use std::path::PathBuf;

use argh::FromArgs;
use framework::config::{
    ExtensionDescription, ProviderDescription, SinkDescription, SourceDescription,
    TransformDescription,
};
use framework::{config, get_version};

#[derive(FromArgs)]
#[argh(description = "Vertex is an all-in-one collector for metrics, logs and traces")]
pub struct RootCommand {
    #[argh(switch, short = 'v', description = "show version")]
    pub version: bool,

    #[argh(
        option,
        short = 'l',
        default = "\"info\".to_string()",
        description = "log level"
    )]
    pub log_level: String,

    #[argh(
        option,
        short = 'c',
        long = "config",
        description = "read configuration from one or more files, wildcard paths are supported"
    )]
    pub configs: Vec<PathBuf>,

    #[argh(
        option,
        short = 't',
        description = "specify how many threads the Tokio runtime will use"
    )]
    pub threads: Option<usize>,

    #[argh(subcommand)]
    pub sub_commands: Option<Commands>,
}

impl RootCommand {
    #![allow(clippy::print_stdout)]
    pub fn show_version(&self) {
        println!("vertex {}", get_version());
    }

    pub fn config_paths_with_formats(&self) -> Vec<config::ConfigPath> {
        config::merge_path_lists(vec![(&self.configs, None)])
            .map(|(path, hint)| config::ConfigPath::File(path, hint))
            .collect::<Vec<_>>()
    }
}

#[derive(Debug, FromArgs)]
#[argh(subcommand)]
pub enum Commands {
    Sources(Sources),
    Transforms(Transforms),
    Sinks(Sinks),
    Extensions(Extensions),
    Providers(Providers),
    Validate(validate::Validate),
}

impl Commands {
    pub fn run(&self) {
        match self {
            Commands::Sources(sources) => sources.run(),
            Commands::Transforms(transforms) => transforms.run(),
            Commands::Sinks(sinks) => sinks.run(),
            Commands::Extensions(extensions) => extensions.run(),
            Commands::Providers(providers) => providers.run(),
            Commands::Validate(validate) => {
                validate.run();
            }
        }
    }
}

macro_rules! impl_list_and_example {
    ($typ:ident, $desc:ident) => {
        impl $typ {
            #![allow(clippy::print_stdout)]
            pub fn run(&self) {
                match &self.name {
                    Some(name) => match $desc::example(&name) {
                        Ok(example) => println!("{}", example.trim()),
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

#[derive(Debug, FromArgs)]
#[argh(subcommand, name = "sources", description = "supported sources")]
pub struct Sources {
    #[argh(positional, description = "source name")]
    name: Option<String>,
}

impl_list_and_example!(Sources, SourceDescription);

#[derive(Debug, FromArgs)]
#[argh(subcommand, name = "transforms", description = "List transforms")]
pub struct Transforms {
    #[argh(positional, description = "transform name")]
    name: Option<String>,
}

impl_list_and_example!(Transforms, TransformDescription);

#[derive(Debug, FromArgs)]
#[argh(subcommand, name = "sinks", description = "List sinks")]
pub struct Sinks {
    #[argh(positional, description = "sink name")]
    name: Option<String>,
}

impl_list_and_example!(Sinks, SinkDescription);

#[derive(Debug, FromArgs)]
#[argh(subcommand, name = "extensions", description = "List extensions")]
pub struct Extensions {
    #[argh(positional, description = "extension name")]
    name: Option<String>,
}

impl_list_and_example!(Extensions, ExtensionDescription);

#[derive(Debug, FromArgs)]
#[argh(subcommand, name = "providers", description = "List providers")]
pub struct Providers {
    #[argh(positional, description = "provider name")]
    name: Option<String>,
}

impl Providers {
    #![allow(clippy::print_stdout)]
    pub fn run(&self) {
        match &self.name {
            Some(name) => match ProviderDescription::example(name) {
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
