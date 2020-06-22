#[macro_use]
extern crate log;

/// Defines `InfoServiceConfig` with all the configuration options for metric collection.
pub mod config;

/// Defines `HeartRateMonitor`, that wraps an `AtomicU64` corresponding to the last valid heartbeat timestamp.
pub mod heartbeat;
pub mod metrics;

/// Defines `PeriodicBackgroundTask` which is used to collect metrics in the background.
pub mod task;

/// Defines `InfoService`, `tower`-based web service which handles metrics collection and heartbeats.
pub mod web;
