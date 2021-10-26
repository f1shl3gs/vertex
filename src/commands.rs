use clap::Parser;
use vertex::config::{SinkDescription, SourceDescription, TransformDescription};

#[derive(Debug, Parser)]
pub enum Commands {
    Describe(Describe)
}

#[derive(Debug, Parser)]
pub struct Describe {
    #[clap(default_value = "all")]
    typ: String,
}

impl Describe {
    pub fn list(&self) {
        match self.typ.as_str() {
            "all" => {
                Self::list_sources();
                Self::list_transforms();
                Self::list_sinks();
            }
            "source" | "src" => {
                Self::list_sources()
            }
            _ => {
                println!("unknown plugin type")
            }
        }
    }

    fn list_sources() {
        for source in inventory::iter::<SourceDescription> {
            let example = SourceDescription::example(source.type_str).unwrap();
            println!("Name: {}\n", source.type_str);
            println!("{}\n", example)
        }
    }

    fn list_transforms() {
        for transform in inventory::iter::<TransformDescription> {
            let example = TransformDescription::example(transform.type_str).unwrap();
            println!("Name: {}\n", transform.type_str);
            println!("{}\n", example)
        }
    }

    fn list_sinks() {
        for sink in inventory::iter::<SinkDescription> {
            let example = SinkDescription::example(sink.type_str).unwrap();
            println!("Name: {}\n", sink.type_str);
            println!("{}\n", example)
        }
    }
}
