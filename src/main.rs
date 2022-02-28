mod commands;

extern crate vertex;

#[cfg(feature = "allocator-mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(any(feature = "allocator-jemalloc", feature = "extensions-jemalloc"))]
#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

extern crate chrono;
extern crate chrono_tz;

use crate::commands::RootCommand;

fn main() {
    let cmd: RootCommand = argh::from_env();

    if let Err(code) = cmd.run() {
        std::process::exit(code)
    }
}
