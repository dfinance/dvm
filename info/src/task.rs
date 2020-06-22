#![warn(missing_docs)]

use std::sync::Arc;
use std::thread::JoinHandle;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

/// Background process. Code executed with `spawn()` will be executed once and exit.
#[derive(Debug)]
pub struct Daemon {
    handler: Option<JoinHandle<()>>,
    enabled: Arc<AtomicBool>,
}

impl Daemon {
    /// Spawn daemon task in a separate thread.
    pub fn spawn<T>(task: T) -> Daemon
    where
        T: FnOnce(&AtomicBool) + Send + Sync + 'static,
    {
        let enabled = Arc::new(AtomicBool::new(true));
        let enabled_ref = enabled.clone();
        let handler = thread::spawn(move || task(&enabled_ref));
        Daemon {
            handler: Some(handler),
            enabled,
        }
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        self.enabled.store(false, Ordering::Relaxed);
        if let Some(handler) = self.handler.take() {
            if let Err(err) = handler.join() {
                warn!("Failed to stop demon [{:?}]", err);
            }
        }
    }
}

/// Signal thread task executed periodically with provided `period`.
#[derive(Debug)]
pub struct PeriodicBackgroundTask {
    daemon: Daemon,
}

impl PeriodicBackgroundTask {
    /// Spawn task.
    pub fn spawn<T>(task: T, period: Duration) -> PeriodicBackgroundTask
    where
        T: Fn() + Sync + Send + 'static,
    {
        let daemon = Daemon::spawn(move |enabled| {
            while enabled.load(Ordering::Relaxed) {
                task();
                thread::sleep(period);
            }
        });
        PeriodicBackgroundTask { daemon }
    }
}
