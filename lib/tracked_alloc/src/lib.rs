use std::alloc::{GlobalAlloc, Layout};
use std::cell::Cell;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Display;
use std::hash::Hasher;
use std::sync::{LazyLock, Mutex};

use backtrace::{Backtrace, BacktraceFmt, BytesOrWideString, PrintFmt};

pub struct TrackedAllocator<T: GlobalAlloc> {
    inner: T,
}

impl<T: GlobalAlloc> TrackedAllocator<T> {
    pub const fn new(inner: T) -> Self {
        TrackedAllocator { inner }
    }
}

#[derive(Clone)]
pub struct TraceInfo {
    pub backtrace: Backtrace,

    pub allocations: usize,
    pub allocated_bytes: usize,
    pub frees: usize,
    pub freed_bytes: usize,
}

impl Display for TraceInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let full = f.alternate();
        let mut backtrace = self.backtrace.clone();
        backtrace.resolve();

        let frames = backtrace.frames();
        let cwd = std::env::current_dir();
        let mut print_path = move |fmt: &mut fmt::Formatter<'_>, path: BytesOrWideString<'_>| {
            let path = path.into_path_buf();
            if !full {
                if let Ok(cwd) = &cwd {
                    if let Ok(suffix) = path.strip_prefix(cwd) {
                        return fmt::Display::fmt(&suffix.display(), fmt);
                    }
                }
            }

            fmt::Display::fmt(&path.display(), fmt)
        };

        let mut f = BacktraceFmt::new(f, PrintFmt::Short, &mut print_path);
        f.add_context()?;
        for frame in frames {
            let symbols = frame.symbols();
            for symbol in symbols {
                if let Some(name) = symbol.name().map(|x| x.to_string()) {
                    let name = name.strip_prefix('<').unwrap_or(&name);
                    if name.starts_with("tracked_alloc::")
                        || name == "__rg_alloc"
                        || name.starts_with("alloc::")
                        || name.starts_with("std::panicking::")
                        || name == "__rust_try"
                        || name == "_start"
                        || name == "__libc_start_main_impl"
                        || name == "__libc_start_call_main"
                        || name.starts_with("std::rt::")
                    {
                        continue;
                    }
                }

                f.frame().backtrace_symbol(frame, symbol)?;
            }
            if symbols.is_empty() {
                f.frame().print_raw(frame.ip(), None, None, None)?;
            }
        }
        f.finish()?;
        Ok(())
    }
}

// key is pointer, value is stack hash
static POINTER_MAP: LazyLock<Mutex<HashMap<usize, u64>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
// key is hash value of the backtrace
static TRACE_MAP: LazyLock<Mutex<HashMap<u64, TraceInfo>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

thread_local! {
    /// Used to avoid recursive alloc/dealloc calls for interior allocation
    static IN_ALLOC: Cell<bool> = const { Cell::new(false) };
}

unsafe impl<T: GlobalAlloc> GlobalAlloc for TrackedAllocator<T> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if IN_ALLOC.get() {
            return self.inner.alloc(layout);
        }

        enter_alloc(|| {
            let size = layout.size();
            let ptr = self.inner.alloc(layout);

            let backtrace = Backtrace::new_unresolved();
            let mut hasher = std::collections::hash_map::DefaultHasher::new();

            backtrace
                .frames()
                .iter()
                .for_each(|frame| hasher.write_u64(frame.ip() as u64));
            let hash = hasher.finish();

            POINTER_MAP.lock().unwrap().insert(ptr as usize, hash);

            TRACE_MAP
                .lock()
                .unwrap()
                .entry(hash)
                .and_modify(|trace| {
                    trace.allocations += 1;
                    trace.allocated_bytes += size;
                })
                .or_insert(TraceInfo {
                    backtrace,
                    allocations: 1,
                    allocated_bytes: size,
                    frees: 0,
                    freed_bytes: 0,
                });

            ptr
        })
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if IN_ALLOC.get() {
            return self.inner.dealloc(ptr, layout);
        }

        enter_alloc(|| {
            if let Some(hash) = POINTER_MAP.lock().unwrap().remove(&(ptr as usize)) {
                if let Some(trace) = TRACE_MAP.lock().unwrap().get_mut(&hash) {
                    trace.frees += 1;
                    trace.freed_bytes += layout.size();
                }
            };

            self.inner.dealloc(ptr, layout)
        })
    }
}

fn enter_alloc<T>(f: impl FnOnce() -> T) -> T {
    let current = IN_ALLOC.get();
    IN_ALLOC.set(true);
    let output = f();
    IN_ALLOC.set(current);
    output
}

/// Iterate all backtraces, and trying to generate something
pub fn report(f: impl FnMut(&TraceInfo)) {
    enter_alloc(|| {
        TRACE_MAP.lock().unwrap().values().for_each(f);
    });
}
