//! Responsible for gathering various metrics from the running dvm process and sending it
//! to the Prometheus instance.

#![warn(missing_docs)]

#[macro_use]
extern crate log;

/// Defines `InfoServiceConfig` with all the configuration options for metric collection.
pub mod config;

/// Defines `HeartRateMonitor`, that wraps an `AtomicU64` corresponding to the last valid heartbeat timestamp.
pub mod heartbeat;

/// Execution metrics.
pub mod metrics;

/// Defines `PeriodicBackgroundTask` which is used to collect metrics in the background.
pub mod task;

/// Defines `InfoService`, `tower`-based web service which handles metrics collection and heartbeats.
pub mod web;

/// Defines `MemoryChecker`
pub mod memory_check;
