use std::sync::RwLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use crate::config::MemoryOptions;
use crate::metrics::execution::get_system_metrics;

/// Dvm memory limits checker.
pub struct MemoryChecker {
    idle_memory: RwLock<Option<usize>>,
    external_cache: Vec<Box<dyn CacheSize>>,
    counter: AtomicUsize,
    max_memory: usize,
    check_interval: usize,
}

impl MemoryChecker {
    /// Constructor.
    pub fn new(options: MemoryOptions, external_cache: Vec<Box<dyn CacheSize>>) -> MemoryChecker {
        MemoryChecker {
            idle_memory: Default::default(),
            external_cache,
            counter: Default::default(),
            max_memory: options.max_dvm_cache_size(),
            check_interval: options.memory_check_period(),
        }
    }

    /// Returns `true` if the amount of memory exceeds the limit.
    pub fn is_limit_exceeded(&self) -> bool {
        let count = self.counter.fetch_add(1, Ordering::SeqCst);

        if count == 0 {
            let idle_memory = get_system_metrics().memory as usize;
            *self
                .idle_memory
                .write()
                .unwrap_or_else(|err| err.into_inner()) = Some(idle_memory);
            false
        } else if count >= self.check_interval {
            self.counter.store(1, Ordering::SeqCst);

            let external_cache_size = self.external_cache.iter().map(|s| s.size()).sum::<usize>();
            let mem = current_memory();

            if external_cache_size > mem {
                warn!("Unexpected cache size.{:?}. Expected that the cache size is less than the process memory usages.", external_cache_size);
                false
            } else {
                let available_memory = mem - external_cache_size;
                let idle_memory = self
                    .idle_memory
                    .read()
                    .unwrap_or_else(|err| err.into_inner())
                    .unwrap_or(0);
                if idle_memory > available_memory {
                    false
                } else {
                    self.max_memory > available_memory - idle_memory
                }
            }
        } else {
            false
        }
    }
}

/// Returns current memory usages.
fn current_memory() -> usize {
    get_system_metrics().memory as usize
}

/// Provide cache size function.
pub trait CacheSize {
    /// Returns cache size.
    fn size(&self) -> usize;
}
