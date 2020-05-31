use std::time::Instant;
use crate::metrics::live_time::{store_metric, ExecutionData, ExecutionResult};

/// Scope metric.
pub struct ScopeMeter {
    name: &'static str,
    instant: Instant,
    result: Option<ExecutionResult>,
}

impl ScopeMeter {
    /// Create a new scope meter.
    pub fn new(name: &'static str) -> ScopeMeter {
        ScopeMeter {
            name,
            instant: Instant::now(),
            result: None,
        }
    }

    /// Set action result.
    pub fn set_result(&mut self, result: ExecutionResult) {
        self.result = Some(result);
    }
}

impl Drop for ScopeMeter {
    fn drop(&mut self) {
        let time = self.instant.elapsed().as_millis();
        store_metric(
            self.name,
            ExecutionData::with_result(time as u64, self.result.take()),
        );
    }
}

#[cfg(test)]
mod test {
    use std::thread;
    use std::time::Duration;
    use crate::metrics::live_time::{ExecutionResult, drain_action_metrics, STORE_METRICS};
    use std::sync::atomic::Ordering;
    use crate::metrics::meter::ScopeMeter;

    #[test]
    fn test_store_metric() {
        STORE_METRICS.store(true, Ordering::Relaxed);

        {
            let mut meter = ScopeMeter::new("test_lunch");
            thread::sleep(Duration::from_secs(2));
            meter.set_result(ExecutionResult::new(true, 200, 10));
        }
        let metric = &drain_action_metrics()["test_lunch"][0];
        assert!(metric.process_time >= 2 * 1000);
        assert_eq!(
            metric.result.as_ref().unwrap(),
            &ExecutionResult::new(true, 200, 10)
        );
    }
}
