use std::alloc::System;

use tracked_alloc::TrackedAllocator;

#[global_allocator]
static GLOBAL: TrackedAllocator<System> = TrackedAllocator::new(System);

fn main() {
    println!("example start");

    tracked_alloc::report(|trace| {
        println!("{}", trace.allocations);
        println!("{}", trace.allocated_bytes);
        println!("{}", trace.frees);
        println!("{}", trace.freed_bytes);
    })
}
