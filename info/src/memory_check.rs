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
            let idle_memory = current_memory();
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
                let used_memory = mem - external_cache_size;
                let idle_memory = self
                    .idle_memory
                    .read()
                    .unwrap_or_else(|err| err.into_inner())
                    .unwrap_or(0);
                if idle_memory > used_memory {
                    false
                } else {
                    self.max_memory < used_memory - idle_memory
                }
            }
        } else {
            false
        }
    }
}

/// Returns current memory usages.
fn current_memory() -> usize {
    (get_system_metrics().memory * 1024) as usize
}

/// Provide cache size function.
pub trait CacheSize {
    /// Returns cache size.
    fn size(&self) -> usize;
}

#[cfg(test)]
mod test {
    use crate::memory_check::{MemoryChecker, CacheSize};
    use crate::config::MemoryOptions;
    use std::rc::Rc;
    use std::cell::RefCell;

    #[derive(Clone)]
    struct FakeCache(Rc<RefCell<Vec<u8>>>);

    impl FakeCache {
        fn new() -> FakeCache {
            FakeCache {
                0: Rc::new(RefCell::new(vec![])),
            }
        }

        fn fill(&self, bytes_count: usize) {
            let mut buffer = self.0.borrow_mut();
            for byte in 0..bytes_count {
                buffer.push(byte as u8);
            }
        }
    }

    impl CacheSize for FakeCache {
        fn size(&self) -> usize {
            self.0.borrow().len()
        }
    }

    #[test]
    fn test_checker() {
        let cache = FakeCache::new();
        let cache_2 = FakeCache::new();
        let checker = MemoryChecker::new(
            MemoryOptions {
                module_cache: 10000,
                memory_check_period: 1000,
                max_dvm_cache_size: 2000,
            },
            vec![Box::new(cache.clone())],
        );

        loop {
            if checker.is_limit_exceeded() {
                break;
            } else {
                cache_2.fill(1024);
                cache.fill(1024);
            }
        }
        assert_eq!(cache_2.size(), 1024 * 1000);
    }
}
