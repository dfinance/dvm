#[macro_use]
extern crate log;

pub mod config;

/// Defines `HeartRateMonitor`, that wraps an `AtomicU64` corresponding to the last valid heartbeat timestamp.
pub mod heartbeat;
pub mod metrics;
pub mod task;
pub mod web;
