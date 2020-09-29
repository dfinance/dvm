use std::sync::{Arc, Mutex};

use libra::prelude::*;
use crate::{RemoveModule, DataSource};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::cell::RefCell;
use std::fmt;
use std::fmt::Formatter;
use dvm_info::memory_check::CacheSize;

/// Cached `DataSource`.
#[derive(Debug, Clone)]
pub struct ModuleCache<D>
where
    D: DataSource,
{
    inner: D,
    cache: Lru<D>,
}

impl<D> ModuleCache<D>
where
    D: DataSource,
{
    /// Create new cached data source with `cache_size` max cache size in bytes.
    pub fn new(inner: D, cache_size: usize) -> ModuleCache<D> {
        ModuleCache {
            inner: inner.clone(),
            cache: Lru::new(inner, cache_size),
        }
    }
}

impl<D: DataSource> CacheSize for ModuleCache<D> {
    fn size(&self) -> usize {
        self.cache.cache_size()
    }
}

impl<D> RemoveModule for ModuleCache<D>
where
    D: DataSource,
{
    fn remove_module(&self, module_id: &ModuleId) {
        self.cache.remove(&module_id);
    }
}

impl<D> RemoteCache for ModuleCache<D>
where
    D: DataSource,
{
    fn get_module(&self, module_id: &ModuleId) -> VMResult<Option<Vec<u8>>> {
        self.cache.read_through(module_id)
    }

    fn get_resource(
        &self,
        address: &AccountAddress,
        tag: &StructTag,
    ) -> PartialVMResult<Option<Vec<u8>>> {
        self.inner.get_resource(address, tag)
    }
}

impl<D> DataSource for ModuleCache<D> where D: DataSource {}

/// Modules lru cache.
#[derive(Debug, Clone)]
pub struct Lru<D: DataSource> {
    inner: Arc<Mutex<LruCache<ModuleId, Vec<u8>>>>,
    source: D,
}

unsafe impl<D: DataSource> Sync for Lru<D> {}

unsafe impl<D: DataSource> Send for Lru<D> {}

impl<D: DataSource> Lru<D> {
    /// Constructor.
    pub fn new(source: D, cache_size: usize) -> Lru<D> {
        Lru {
            inner: Arc::new(Mutex::new(LruCache::new(cache_size))),
            source,
        }
    }

    /// Reads module through cache.
    pub fn read_through(&self, module_id: &ModuleId) -> VMResult<Option<Vec<u8>>> {
        let mut cache = self.inner.lock().unwrap_or_else(|err| err.into_inner());
        let key = Key::new(module_id.to_owned());
        if let Some(entry) = cache.get(&key) {
            Ok(Some(entry.as_ref().to_owned()))
        } else if let Some(val) = self.source.get_module(key.as_ref())? {
            cache.put(key, val.clone());
            Ok(Some(val))
        } else {
            Ok(None)
        }
    }

    /// Remove module from cache.
    pub fn remove(&self, module_id: &ModuleId) {
        let mut cache = self.inner.lock().unwrap_or_else(|err| err.into_inner());
        cache.remove(&Key::new(module_id.to_owned()));
    }

    /// Returns the cache binary size.
    pub fn cache_size(&self) -> usize {
        let cache = self.inner.lock().unwrap_or_else(|err| err.into_inner());
        cache.cache_size
    }
}

type EntryRef<K, V> = Rc<RefCell<Option<Entry<K, V>>>>;

struct LruCache<K: Eq + Hash + Clone, V: BinarySize> {
    map: HashMap<Key<K>, Entry<K, V>>,
    cache_size: usize,
    capacity: usize,
    first: EntryRef<K, V>,
    last: EntryRef<K, V>,
}

impl<K: Eq + Hash + Clone + fmt::Debug, V: BinarySize + fmt::Debug> LruCache<K, V> {
    fn new(capacity: usize) -> LruCache<K, V> {
        LruCache {
            map: Default::default(),
            cache_size: Default::default(),
            capacity,
            first: Rc::new(RefCell::new(None)),
            last: Rc::new(RefCell::new(None)),
        }
    }

    fn put(&mut self, key: Key<K>, value: V) {
        let key = key.into_shared();
        let mut value = Entry::new(key.clone(), value);

        let first = self.first.borrow_mut().take();
        if let Some(mut first) = first {
            first.link_prev(Some(value.clone()));
            value.link_next(Some(first));
            *self.first.borrow_mut() = Some(value.clone());
        } else {
            *self.first.borrow_mut() = Some(value.clone());
        }

        if self.last.borrow().is_none() {
            *self.last.borrow_mut() = Some(value.clone());
        }

        self.cache_size += value.size();
        self.map.insert(key, value);

        self.trim_to_size();
    }

    fn get(&mut self, key: &Key<K>) -> Option<Rc<V>> {
        if let Some(entry) = self.map.get_mut(&key) {
            let mut entry = entry.clone();
            self.safe_unlink(&mut entry);

            let mut first = self.first.borrow_mut().take();
            if let Some(first) = &mut first {
                first.link_prev(Some(entry.clone()));
            }

            entry.link_next(first);
            *self.first.borrow_mut() = Some(entry.clone());
            Some(entry.val)
        } else {
            None
        }
    }

    fn trim_to_size(&mut self) {
        while self.cache_size > self.capacity {
            let key = self.last.borrow().as_ref().map(|l| l.key.clone());
            if let Some(key) = key {
                self.remove(&key);
            } else {
                break;
            }
        }
    }

    fn safe_unlink(&mut self, entry: &mut Entry<K, V>) {
        let first = self.first.borrow().as_ref().cloned();
        if let Some(first) = first {
            if entry == &first {
                *self.first.borrow_mut() = first.next.borrow().as_ref().cloned();
            }
        }

        let last = self.last.borrow().as_ref().cloned();
        if let Some(last) = last {
            if entry == &last {
                *self.last.borrow_mut() = last.prev.borrow().as_ref().cloned();
            }
        }

        entry.safe_unlink();
    }

    fn remove(&mut self, key: &Key<K>) {
        if let Some(mut entry) = self.map.remove(key) {
            self.safe_unlink(&mut entry);
            self.cache_size -= entry.size();
        }
    }

    #[cfg(test)]
    fn size(&self) -> usize {
        self.cache_size
    }
}

impl<K: Eq + Hash + Clone + fmt::Debug, V: BinarySize + fmt::Debug> fmt::Debug for LruCache<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[map={:?}; cache_size={:?}; capacity={:?}; ",
            self.map, self.cache_size, self.capacity
        )?;
        if let Some(first) = self.first.borrow().as_ref() {
            write!(f, "first={:?}; ", first.key)?;
        } else {
            write!(f, "first=None; ")?;
        }

        if let Some(last) = self.last.borrow().as_ref() {
            write!(f, "last={:?}]", last.key)
        } else {
            write!(f, "last=None]")
        }
    }
}

#[derive(Eq)]
enum Key<K: Eq + Hash + Clone> {
    Shared(Rc<K>),
    Owned(K),
}

impl<K: Eq + Hash + Clone> AsRef<K> for Key<K> {
    fn as_ref(&self) -> &K {
        match self {
            Key::Shared(r) => r.as_ref(),
            Key::Owned(r) => r,
        }
    }
}

impl<K: Eq + Hash + Clone> PartialEq for Key<K> {
    fn eq(&self, other: &Self) -> bool {
        self.as_ref().eq(&other.as_ref())
    }
}

impl<K: Eq + Hash + Clone> Hash for Key<K> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state)
    }
}

impl<K: Eq + Hash + Clone> Key<K> {
    fn new(key: K) -> Self {
        Key::Owned(key)
    }

    fn into_shared(self) -> Key<K> {
        match self {
            Key::Shared(key) => Key::Shared(key),
            Key::Owned(key) => Key::Shared(Rc::new(key)),
        }
    }
}

impl<K: Eq + Hash + Clone> Clone for Key<K> {
    fn clone(&self) -> Self {
        match self {
            Key::Shared(key) => Key::Shared(key.clone()),
            Key::Owned(key) => Key::Owned(key.clone()),
        }
    }
}

impl<K: Eq + Hash + Clone + fmt::Debug> fmt::Debug for Key<K> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.as_ref())
    }
}

struct Entry<K: Eq + Hash + Clone, V: BinarySize> {
    key: Key<K>,
    val: Rc<V>,
    prev: EntryRef<K, V>,
    next: EntryRef<K, V>,
}

impl<K: Eq + Hash + Clone, V: BinarySize> PartialEq for Entry<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.key.eq(&other.key)
    }
}

impl<K: Eq + Hash + Clone, V: BinarySize> Entry<K, V> {
    fn new(key: Key<K>, value: V) -> Entry<K, V> {
        Entry {
            key,
            val: Rc::new(value),
            prev: Rc::new(RefCell::new(None)),
            next: Rc::new(RefCell::new(None)),
        }
    }

    fn link_prev(&mut self, prev: Option<Entry<K, V>>) {
        *self.prev.borrow_mut() = prev;
    }

    fn link_next(&mut self, next: Option<Entry<K, V>>) {
        *self.next.borrow_mut() = next;
    }

    fn safe_unlink(&mut self) {
        let next = self.next.borrow_mut().take();
        let prev = self.prev.borrow_mut().take();

        if let Some(next) = next {
            if let Some(prev) = prev {
                *next.prev.borrow_mut() = Some(prev.clone());
                *prev.next.borrow_mut() = Some(next);
            } else {
                *next.prev.borrow_mut() = None;
            }
        } else if let Some(prev) = prev {
            *prev.next.borrow_mut() = None;
        }
    }

    fn unlink(&mut self) {
        self.prev.borrow_mut().take();
        self.next.borrow_mut().take();
    }
}

impl<K: Eq + Hash + Clone, V: BinarySize> BinarySize for Entry<K, V> {
    fn size(&self) -> usize {
        self.val.size()
    }
}

impl<K: Eq + Hash + Clone, V: BinarySize> Clone for Entry<K, V> {
    fn clone(&self) -> Self {
        Entry {
            key: self.key.clone(),
            val: self.val.clone(),
            prev: self.prev.clone(),
            next: self.next.clone(),
        }
    }
}

/// Provides a binary size info.
pub trait BinarySize {
    /// Result size of self.
    fn size(&self) -> usize;
}

impl BinarySize for Vec<u8> {
    fn size(&self) -> usize {
        self.len()
    }
}

impl<K: Eq + Hash + Clone + fmt::Debug, V: BinarySize + fmt::Debug> fmt::Debug for Entry<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[key={:?}; value={:?}; ", self.key, self.val.as_ref())?;
        if let Some(prev) = self.prev.borrow().as_ref() {
            write!(f, "prev={:?}; ", prev.key)?;
        } else {
            write!(f, "prev=None; ")?;
        }

        if let Some(next) = self.next.borrow().as_ref() {
            write!(f, "next={:?}]", next.key)
        } else {
            write!(f, "next=None]")
        }
    }
}

impl<K: Eq + Hash + Clone, V: BinarySize> Drop for LruCache<K, V> {
    fn drop(&mut self) {
        self.map.iter_mut().for_each(|(_, v)| v.unlink());
    }
}

#[cfg(test)]
mod tests {
    use crate::module_cache::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    impl BinarySize for i32 {
        fn size(&self) -> usize {
            4
        }
    }

    #[derive(Debug)]
    pub struct Value {
        acc: Rc<AtomicUsize>,
    }

    impl Value {
        pub fn new(acc: Rc<AtomicUsize>) -> Value {
            acc.store(acc.load(Ordering::SeqCst) + 1, Ordering::SeqCst);
            Value { acc }
        }
    }

    impl Drop for Value {
        fn drop(&mut self) {
            self.acc
                .store(self.acc.load(Ordering::SeqCst) - 1, Ordering::SeqCst);
        }
    }

    impl BinarySize for Value {
        fn size(&self) -> usize {
            4
        }
    }

    #[test]
    fn test_overflow() {
        let capacity = 20;
        let mut cache = LruCache::new(capacity);

        let acc = Rc::new(AtomicUsize::new(0));
        for i in 0..1000 {
            cache.put(Key::new(i), Value::new(acc.clone()));
            assert!(cache.size() <= capacity);
        }

        assert_eq!(acc.load(Ordering::SeqCst), 5);
        assert_eq!(cache.size(), 20);

        for i in 996..1000 {
            assert!(cache.get(&Key::new(i)).is_some());
        }

        for i in 0..995 {
            assert!(cache.get(&Key::new(i)).is_none());
        }
    }

    #[test]
    fn test_remove() {
        let capacity = 20;
        let mut cache = LruCache::new(capacity);

        let acc = Rc::new(AtomicUsize::new(0));
        for i in 0..6 {
            cache.put(Key::new(i), Value::new(acc.clone()));
            assert!(cache.size() <= capacity);
        }
        assert_eq!(cache.size(), 20);
        assert_eq!(acc.load(Ordering::SeqCst), 5);

        cache.remove(&Key::new(3));
        assert_eq!(cache.size(), 16);
        assert_eq!(acc.load(Ordering::SeqCst), 4);

        cache.remove(&Key::new(1));
        assert_eq!(cache.size(), 12);
        assert_eq!(acc.load(Ordering::SeqCst), 3);

        cache.remove(&Key::new(5));
        assert_eq!(cache.size(), 8);
        assert_eq!(acc.load(Ordering::SeqCst), 2);

        cache.remove(&Key::new(2));
        assert_eq!(acc.load(Ordering::SeqCst), 1);

        cache.remove(&Key::new(4));
        assert_eq!(acc.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_get() {
        let capacity = 20;
        let mut cache = LruCache::new(capacity);

        let acc = Rc::new(AtomicUsize::new(0));
        for i in 0..1000 {
            cache.put(Key::new(i), Value::new(acc.clone()));
            cache.get(&Key::new(0));
            cache.get(&Key::new(1));
            cache.get(&Key::new(2));
        }

        assert!(cache.get(&Key::new(0)).is_some());
        assert!(cache.get(&Key::new(1)).is_some());
        assert!(cache.get(&Key::new(2)).is_some());

        for i in 3..998 {
            assert!(cache.get(&Key::new(i)).is_none());
        }

        assert!(cache.get(&Key::new(998)).is_some());
        assert!(cache.get(&Key::new(999)).is_some());
    }
}
