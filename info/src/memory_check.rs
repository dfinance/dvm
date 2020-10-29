use std::sync::atomic::{AtomicUsize, Ordering};
use crate::config::MemoryOptions;

/// Dvm memory limits checker.
pub struct MemoryChecker {
    counter: AtomicUsize,
    check_interval: usize,
}

impl MemoryChecker {
    /// Constructor.
    pub fn new(options: MemoryOptions) -> MemoryChecker {
        MemoryChecker {
            counter: Default::default(),
            check_interval: options.memory_check_period(),
        }
    }

    /// Returns `true` if the amount of memory exceeds the limit.
    pub fn is_limit_exceeded(&self) -> bool {
        let count = self.counter.fetch_add(1, Ordering::SeqCst);

        if count >= self.check_interval {
            self.counter.store(1, Ordering::SeqCst);
            true
        } else {
            false
        }
    }
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
}
