mod launch;
mod top;
mod validate;
mod vtl;

use launch::RootCommand;

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(feature = "jemalloc")]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[cfg(feature = "snmalloc")]
#[global_allocator]
static ALLOC: snmalloc_rs::SnMalloc = snmalloc_rs::SnMalloc;

#[cfg(feature = "scudo")]
#[global_allocator]
static SCUDO_ALLOCATOR: scudo::GlobalScudoAllocator = scudo::GlobalScudoAllocator;

#[cfg(feature = "tracked_allocator")]
#[global_allocator]
static TRACKED_ALLOCATOR: vertex::common::tracked_allocator::TrackedAllocator =
    vertex::common::tracked_allocator::TrackedAllocator::new();

fn main() {
    let cmd: RootCommand = argh::from_env();

    if let Err(code) = cmd.run() {
        std::process::exit(code)
    }
}
