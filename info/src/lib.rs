#[macro_use]
extern crate log;

pub mod config;

/// Defines `HeartRateMonitor`, that wraps an `AtomicU64` corresponding to the last valid heartbeat timestamp.
pub mod heartbeat;
pub mod metrics;

/// Defines `PeriodicBackgroundTask` which is used to collect metrics in the background.
pub mod task;
pub mod web;
