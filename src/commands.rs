#![allow(clippy::print_stdout)] // tests

use argh::FromArgs;
use vertex::config::{
    ExtensionDescription, ProviderDescription, SinkDescription, SourceDescription,
    TransformDescription,
};

#[derive(FromArgs)]
#[argh(description = "Vertex is an All-in-one collector for metrics, logs and traces")]
pub struct RootCommand {
    #[argh(
        option,
        short = 'c',
        default = "String::from(\"/etc/vertex/vertex.yml\")",
        description = "specify config file"
    )]
    pub config: String,

    #[argh(
        option,
        short = 't',
        description = "specify how many threads the Tokio runtime will use"
    )]
    pub threads: Option<usize>,

    #[argh(subcommand)]
    pub sub_commands: Option<Commands>,
}

#[derive(Debug, FromArgs)]
#[argh(subcommand)]
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

/*struct Component<T, D> {

    _t: PhantomData<T>,
    _p: PhantomData<D>,
}

impl<T, D> Component<T, D> {
    fn run() {

    }
}*/

#[derive(Debug, FromArgs)]
#[argh(subcommand, name = "sources", description = "supported sources")]
pub struct Sources {
    #[argh(positional, description = "source name")]
    name: Option<String>,
}

impl_list_and_example!(Sources, SourceDescription);

#[derive(Debug, FromArgs)]
#[argh(subcommand, name = "transforms", description = "supported transforms")]
pub struct Transforms {
    #[argh(positional, description = "transform name")]
    name: Option<String>,
}

impl_list_and_example!(Transforms, TransformDescription);

#[derive(Debug, FromArgs)]
#[argh(subcommand, name = "sinks", description = "supported sinks")]
pub struct Sinks {
    #[argh(positional, description = "sink name")]
    name: Option<String>,
}

impl_list_and_example!(Sinks, SinkDescription);

#[derive(Debug, FromArgs)]
#[argh(subcommand, name = "extensions", description = "supported extensions")]
pub struct Extensions {
    #[argh(positional, description = "extension name")]
    name: Option<String>,
}

impl_list_and_example!(Extensions, ExtensionDescription);

#[derive(Debug, FromArgs)]
#[argh(subcommand, name = "providers", description = "supported providers")]
pub struct Providers {
    #[argh(positional, description = "provider name")]
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
