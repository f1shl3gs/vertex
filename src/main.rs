mod launch;
mod top;
mod validate;

use launch::RootCommand;

extern crate vertex;

fn main() {
    let cmd: RootCommand = argh::from_env();

    if let Err(code) = cmd.run() {
        std::process::exit(code)
    }
}
