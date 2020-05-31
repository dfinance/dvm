use crate::metrics::live_time::{STORE_METRICS, drain_action_metrics, get_sys_metrics};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use std::thread;
use std::sync::atomic::Ordering;
use crate::metrics::metric::Metrics;
use crate::task::FixedDelayDemon;

/// Metrics collector.
#[derive(Debug, Clone)]
pub struct MetricsCollector {
    metrics: Arc<RwLock<Metrics>>,
    task: Arc<FixedDelayDemon>,
}

impl MetricsCollector {
    /// Create a new metrics collector.
    /// `interval` is metric interval.
    pub fn new(interval: Duration) -> MetricsCollector {
        STORE_METRICS.store(true, Ordering::Relaxed);
        let metrics = Arc::new(RwLock::new(Default::default()));
        let task = MetricsCollector::start_collector(interval, metrics.clone());
        MetricsCollector {
            metrics,
            task: Arc::new(task),
        }
    }

    /// Get current metrics.
    pub fn get_metrics(&self) -> Metrics {
        self.metrics.read().unwrap().clone()
    }

    /// Start collecting process.
    fn start_collector(interval: Duration, metrics: Arc<RwLock<Metrics>>) -> FixedDelayDemon {
        FixedDelayDemon::spawn(
            move || {
                let new_metric = Metrics::calculate(get_sys_metrics(), drain_action_metrics());
                *metrics.write().unwrap() = new_metric;
                thread::sleep(interval);
            },
            interval,
        )
    }
}

impl Drop for MetricsCollector {
    fn drop(&mut self) {
        STORE_METRICS.store(false, Ordering::Relaxed);
    }
}
