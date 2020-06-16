use std::sync::Arc;
use std::time::{SystemTime, Duration};
use std::sync::atomic::Ordering;
use std::sync::atomic::AtomicU64;

/// Heart rate monitor.
#[derive(Debug, Clone)]
pub struct HeartRateMonitor {
    last_heartbeat: Arc<AtomicU64>,
    max_pause: Duration,
}

impl HeartRateMonitor {
    /// Create a new heartbeat monitor with the given maximum pause duration between heartbeats.
    pub fn new(max_pause: Duration) -> HeartRateMonitor {
        let interval = HeartRateMonitor {
            last_heartbeat: Arc::new(AtomicU64::new(0)),
            max_pause,
        };

        interval.beat();
        interval
    }

    pub fn beat(&self) {
        let sys_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        self.last_heartbeat
            .store(sys_time.as_secs(), Ordering::Relaxed);
    }

    pub fn last_heartbeat_interval(&self) -> Duration {
        let sys_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        sys_time - Duration::from_secs(self.last_heartbeat.load(Ordering::Relaxed))
    }

    pub fn is_alive(&self) -> bool {
        self.max_pause >= self.last_heartbeat_interval()
    }
}
