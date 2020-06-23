#![warn(missing_docs)]

use std::sync::{Arc, RwLock};
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

use crate::metrics::execution::{drain_action_metrics, STORE_METRICS};
use crate::metrics::metric::Metrics;
use crate::task::PeriodicBackgroundTask;

/// Metrics collector.
#[derive(Debug, Clone)]
pub struct MetricsCollector {
    inner: Arc<CollectorState>,
}

/// Metrics collector state. Wraps collected metrics and periodic task.
#[derive(Debug)]
struct CollectorState {
    metrics: Arc<RwLock<Metrics>>,
    task: PeriodicBackgroundTask,
}

impl MetricsCollector {
    /// Create a `MetricsCollector` which fires once every `time_between_collects`.
    pub fn new(time_between_collects: Duration) -> MetricsCollector {
        STORE_METRICS.store(true, Ordering::Relaxed);
        let metrics = Arc::new(RwLock::new(Default::default()));
        let task = MetricsCollector::start_collector(time_between_collects, metrics.clone());
        MetricsCollector {
            inner: Arc::new(CollectorState { metrics, task }),
        }
    }

    /// Get current metrics.
    pub fn get_metrics(&self) -> Metrics {
        self.inner.metrics.read().unwrap().clone()
    }

    /// Start collecting process.
    fn start_collector(
        time_between_collects: Duration,
        metrics: Arc<RwLock<Metrics>>,
    ) -> PeriodicBackgroundTask {
        PeriodicBackgroundTask::spawn(
            move || {
                let new_metric = Metrics::calculate(drain_action_metrics());
                *metrics.write().unwrap() = new_metric;
                thread::sleep(time_between_collects);
            },
            time_between_collects,
        )
    }
}

impl Drop for CollectorState {
    fn drop(&mut self) {
        STORE_METRICS.store(false, Ordering::Relaxed);
    }
}
