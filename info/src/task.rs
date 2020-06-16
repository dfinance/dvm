use std::sync::Arc;
use std::thread::JoinHandle;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

/// Demon task.
#[derive(Debug)]
pub struct Demon {
    handler: Option<JoinHandle<()>>,
    is_run: Arc<AtomicBool>,
}

impl Demon {
    /// Spawn demon task.
    pub fn spawn<T>(task: T) -> Demon
    where
        T: FnOnce(&AtomicBool) + Send + Sync + 'static,
    {
        let is_run = Arc::new(AtomicBool::new(true));
        let is_run_cp = is_run.clone();
        let handler = thread::spawn(move || task(&is_run_cp));
        Demon {
            handler: Some(handler),
            is_run,
        }
    }
}

impl Drop for Demon {
    fn drop(&mut self) {
        self.is_run.store(false, Ordering::Relaxed);
        if let Some(handler) = self.handler.take() {
            if let Err(err) = handler.join() {
                warn!("Failed to stop demon [{:?}]", err);
            }
        }
    }
}

/// Signal thread task with fixed delay.
#[derive(Debug)]
pub struct FixedDelayDemon {
    demon: Demon,
}

impl FixedDelayDemon {
    /// Spawn task.
    pub fn spawn<T>(task: T, delay: Duration) -> FixedDelayDemon
    where
        T: Fn() + Sync + Send + 'static,
    {
        let demon = Demon::spawn(move |is_run| {
            while is_run.load(Ordering::Relaxed) {
                task();
                thread::sleep(delay);
            }
        });
        FixedDelayDemon { demon }
    }
}
