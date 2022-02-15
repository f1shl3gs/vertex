#![deny(warnings, missing_docs)]

//! This crate provides:
//! - `TelemetryLayer`, a generic tracing layer that handles publishing spans and events to arbitrary backends
//! - Utilities for implementing distributed tracing for arbitrary backends
//!
//! As a tracing layer, `TelemetryLayer` can be composed with other layers to provide stdout logging, filtering, etc.
//!
//! This crate is primarily intended to be used by people implementing their own backends.
//! A concrete implementation using honeycomb.io as a backend is available in the [`tracing-honeycomb` crate](https://crates.io/crates/tracing-honeycomb).

mod layer;
mod telemetry;
mod trace;

pub use crate::layer::TelemetryLayer;
pub use crate::telemetry::Telemetry;
pub use crate::trace::{current_dist_trace_ctx, register_dist_tracing_root, TraceContextError};
