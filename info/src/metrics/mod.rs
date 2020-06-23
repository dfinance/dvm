/// Defines `MetricsCollector` which handles background process of collecting.
pub mod collector;
/// Gathers metrics for the process (like cpu usage or memory).
pub mod execution;
/// Defines `ScopeMeter` which handles metric recording.
pub mod meter;
/// Defines `Metrics` struct and all required aggregates.
pub mod metric;
/// Helper functions to work with Prometheus.
pub mod prometheus;
