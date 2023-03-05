use vertex::launch::RootCommand;

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(any(feature = "jemalloc", feature = "extensions-jemalloc"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

fn main() {
    let cmd: RootCommand = argh::from_env();

    if let Err(code) = cmd.run() {
        std::process::exit(code)
    }
}
