use std::path::PathBuf;

use argh::FromArgs;
use exitcode::ExitCode;
use value::Value;
use vtl::{Diagnostic, TargetValue};

#[derive(FromArgs, Debug)]
#[argh(
    subcommand,
    name = "vtl",
    description = "Run a VTL script",
    help_triggers("-h", "--help")
)]
pub struct Vtl {
    #[argh(positional, description = "which file to run")]
    path: PathBuf,
}

#[allow(clippy::print_stdout)]
impl Vtl {
    pub fn run(&self) -> Result<(), ExitCode> {
        let script = match std::fs::read_to_string(&self.path) {
            Ok(content) => content,
            Err(_err) => return Err(exitcode::OSFILE),
        };

        let program = match vtl::compile(&script) {
            Ok(program) => program,
            Err(err) => {
                let snippets = Diagnostic::new(script).snippets(err);
                println!("{snippets}");
                return Err(exitcode::SOFTWARE);
            }
        };

        let mut target = TargetValue {
            metadata: Value::Object(Default::default()),
            value: Value::Object(Default::default()),
        };
        match program.run(&mut target) {
            Ok(_result) => {
                println!("{target:#?}");
            }
            Err(err) => {
                let output = Diagnostic::new(script).snippets(err);
                println!("{output}");
                return Err(exitcode::SOFTWARE);
            }
        }

        Ok(())
    }
}
