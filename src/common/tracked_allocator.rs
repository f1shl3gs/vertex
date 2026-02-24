use std::alloc::{GlobalAlloc, Layout};
use std::sync::atomic::{AtomicUsize, Ordering};

static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static DEALLOCATED: AtomicUsize = AtomicUsize::new(0);

static ALLOC: AtomicUsize = AtomicUsize::new(0);
static DEALLOC: AtomicUsize = AtomicUsize::new(0);

pub struct TrackedAllocator(std::alloc::System);

impl TrackedAllocator {
    pub const fn new() -> Self {
        Self(std::alloc::System)
    }
}

unsafe impl GlobalAlloc for TrackedAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOC.fetch_add(1, Ordering::Relaxed);
        ALLOCATED.fetch_add(layout.size(), Ordering::AcqRel);
        unsafe { self.0.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        DEALLOC.fetch_add(1, Ordering::Relaxed);
        DEALLOCATED.fetch_add(layout.size(), Ordering::AcqRel);
        unsafe { self.0.dealloc(ptr, layout) }
    }
}

pub fn statistics() -> (usize, usize, usize, usize) {
    let alloc = ALLOC.load(Ordering::Acquire);
    let allocated = ALLOCATED.load(Ordering::Acquire);
    let dealloc = DEALLOC.load(Ordering::Acquire);
    let deallocated = DEALLOCATED.load(Ordering::Acquire);

    (alloc, allocated, dealloc, deallocated)
}
