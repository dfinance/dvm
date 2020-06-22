use crate::metrics::live_time::{STORE_METRICS, drain_action_metrics};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use std::thread;
use std::sync::atomic::Ordering;
use crate::metrics::metric::Metrics;
use crate::task::PeriodicBackgroundTask;

/// Metrics collector.
#[derive(Debug, Clone)]
pub struct MetricsCollector {
    inner: Arc<MetricsInner>,
}

/// Metrics collector state.
#[derive(Debug)]
struct MetricsInner {
    metrics: Arc<RwLock<Metrics>>,
    task: PeriodicBackgroundTask,
}

impl MetricsCollector {
    /// Create a new metrics collector.
    /// `interval` is metric interval.
    pub fn new(interval: Duration) -> MetricsCollector {
        STORE_METRICS.store(true, Ordering::Relaxed);
        let metrics = Arc::new(RwLock::new(Default::default()));
        let task = MetricsCollector::start_collector(interval, metrics.clone());
        MetricsCollector {
            inner: Arc::new(MetricsInner { metrics, task }),
        }
    }

    /// Get current metrics.
    pub fn get_metrics(&self) -> Metrics {
        self.inner.metrics.read().unwrap().clone()
    }

    /// Start collecting process.
    fn start_collector(
        period_between_collects: Duration,
        metrics: Arc<RwLock<Metrics>>,
    ) -> PeriodicBackgroundTask {
        PeriodicBackgroundTask::spawn(
            move || {
                let new_metric = Metrics::calculate(drain_action_metrics());
                *metrics.write().unwrap() = new_metric;
                thread::sleep(period_between_collects);
            },
            period_between_collects,
        )
    }
}

impl Drop for MetricsInner {
    fn drop(&mut self) {
        STORE_METRICS.store(false, Ordering::Relaxed);
    }
}
