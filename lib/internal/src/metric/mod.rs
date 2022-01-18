mod recorder;

// re-export
pub use recorder::InternalRecorder;

use once_cell::sync::OnceCell;
use std::io::{Error, ErrorKind};

// Rust std library provide OnceCell but it's unstable,
// maybe in the future we can remove the dependence `once_cell`
static GLOBAL_RECORDER: OnceCell<InternalRecorder> = OnceCell::new();

fn metrics_enabled() -> bool {
    !matches!(std::env::var("DISABLE_INTERNAL_METRICS_CORE"), Ok(x) if x == "true")
}

pub fn init_global() -> Result<(), Error> {
    // An escape hatch to allow disabling internal metrics core. May be used for performance
    // reasons. This is a hidden and undocumented functionality
    if !metrics_enabled() {
        metrics::set_boxed_recorder(Box::new(metrics::NoopRecorder))
            .map_err(|_| Error::from(ErrorKind::AlreadyExists))?;
        // info!("Internal metrics core is disabled");
        return Ok(());
    }

    // initialize the recorder
    let recorder = InternalRecorder::new();

    GLOBAL_RECORDER
        .set(recorder)
        .map_err(|_| Error::from(ErrorKind::AlreadyExists))?;

    metrics::set_recorder(get_global().unwrap()).map_err(|_| Error::from(ErrorKind::NotFound))
}

pub fn get_global() -> Result<&'static InternalRecorder, Error> {
    GLOBAL_RECORDER
        .get()
        .ok_or_else(|| Error::from(ErrorKind::NotFound))
}
