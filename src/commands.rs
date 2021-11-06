use clap::Parser;
use vertex::config::{ExtensionDescription, SinkDescription, SourceDescription, TransformDescription};

#[derive(Debug, Parser)]
pub enum Commands {
    Sources(Sources),
    Transforms(Transforms),
    Sinks(Sinks),
    Extensions(Extensions),
}

/// TODO: the run implement is dummy, maybe we can simplify it with trait
#[derive(Debug, Parser)]
pub struct Sources {
    name: Option<String>,
}

impl Sources {
    pub fn run(&self) {
        match &self.name {
            Some(name) => {
                for source in inventory::iter::<SourceDescription> {
                    if source.type_str == name {
                        let example = SourceDescription::example(source.type_str).unwrap();
                        println!("Name: {}\n", source.type_str);
                        println!("{:?}\n", example)
                    }
                }
            }
            _ => {
                for source in inventory::iter::<SourceDescription> {
                    println!("{}", source.type_str);
                }
            }
        }
    }
}

#[derive(Debug, Parser)]
pub struct Transforms {
    name: Option<String>,
}

impl Transforms {
    pub fn run(&self) {
        match &self.name {
            Some(name) => {
                for transform in inventory::iter::<TransformDescription> {
                    if transform.type_str == name {
                        let example = TransformDescription::example(transform.type_str).unwrap();
                        println!("Name: {}\n", transform.type_str);
                        println!("{:?}\n", example)
                    }
                }
            }

            _ => {
                for transform in inventory::iter::<TransformDescription> {
                    println!("{}", transform.type_str);
                }
            }
        }
    }
}


#[derive(Debug, Parser)]
pub struct Sinks {
    name: Option<String>,
}

impl Sinks {
    pub fn run(&self) {
        match &self.name {
            Some(name) => {
                for sink in inventory::iter::<SinkDescription> {
                    if sink.type_str == name {
                        let example = SinkDescription::example(sink.type_str).unwrap();
                        println!("Name: {}\n", sink.type_str);
                        println!("{:?}\n", example)
                    }
                }
            }

            _ => {
                for sink in inventory::iter::<SinkDescription> {
                    println!("{}", sink.type_str);
                }
            }
        }
    }
}

#[derive(Debug, Parser)]
pub struct Extensions {
    name: Option<String>,
}

impl Extensions {
    pub fn run(&self) {
        match &self.name {
            Some(name) => {
                for extension in inventory::iter::<ExtensionDescription> {
                    if extension.type_str == name {
                        let example = ExtensionDescription::example(extension.type_str).unwrap();
                        println!("Name: {}\n", extension.type_str);
                        println!("{:?}\n", example)
                    }
                }
            }

            _ => {
                for sink in ExtensionDescription::types() {
                    println!("{}", sink)
                }
            }
        }
    }
}
