use std::sync::{Arc, Mutex};
use lru::LruCache;

use libra::prelude::*;
use crate::{Clear, DataSource};

/// Cached `DataSource`.
#[derive(Debug, Clone)]
pub struct ModuleCache<D>
where
    D: DataSource,
{
    inner: D,
    cache: Arc<Mutex<LruCache<ModuleId, Vec<u8>>>>,
}

impl<D> ModuleCache<D>
where
    D: DataSource,
{
    /// Create new cached data source with `cache_size` max number of entries in cache.
    pub fn new(inner: D, cache_size: usize) -> ModuleCache<D> {
        ModuleCache {
            inner,
            cache: Arc::new(Mutex::new(LruCache::new(cache_size))),
        }
    }
}

impl<D> Clear for ModuleCache<D>
where
    D: DataSource,
{
    fn clear(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.clear();
        self.inner.clear();
    }
}

impl<D> RemoteCache for ModuleCache<D>
where
    D: DataSource,
{
    fn get_module(&self, module_id: &ModuleId) -> VMResult<Option<Vec<u8>>> {
        let module = {
            let mut cache = self.cache.lock().unwrap();
            cache.get(module_id).map(|m| m.to_vec())
        };
        if let Some(module) = module {
            Ok(Some(module))
        } else {
            let module = self.inner.get_module(module_id)?;
            if let Some(module) = module {
                let mut cache = self.cache.lock().unwrap();
                cache.put(module_id.to_owned(), module.to_vec());
                Ok(Some(module))
            } else {
                Ok(None)
            }
        }
    }

    fn get_resource(
        &self,
        address: &AccountAddress,
        tag: &TypeTag,
    ) -> PartialVMResult<Option<Vec<u8>>> {
        self.inner.get_resource(address, tag)
    }
}

impl<D> DataSource for ModuleCache<D> where D: DataSource {}
