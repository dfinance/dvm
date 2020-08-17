use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::Error;

use libra::prelude::*;

use crate::{Clear, DataSource};

/// `StateView` implementation to be used in test_kit.
#[derive(Debug, Clone, Default)]
pub struct MockDataSource {
    data: Arc<Mutex<HashMap<AccessPath, Vec<u8>>>>,
}

impl MockDataSource {
    /// Proxy to default() constructor.
    pub fn new() -> MockDataSource {
        MockDataSource {
            data: Arc::new(Mutex::new(Default::default())),
        }
    }

    /// Create `MockDataSource` with `write_set` applied.
    pub fn with_write_set(write_set: WriteSet) -> MockDataSource {
        let ds = MockDataSource::new();
        ds.merge_write_set(write_set);
        ds
    }

    /// Extract `WriteSet` from internal state.
    pub fn to_write_set(&self) -> Result<WriteSet, Error> {
        let data = self.data.lock().unwrap();
        let ws = data
            .iter()
            .map(|(path, blob)| (path.clone(), WriteOp::Value(blob.clone())))
            .collect();
        WriteSetMut::new(ws).freeze()
    }

    /// Add module to internal state.
    pub fn publish_module(&self, module: Vec<u8>) -> Result<ModuleId, Error> {
        let id = CompiledModule::deserialize(&module)
            .map_err(|e| e.finish(Location::Undefined).into_vm_status())?
            .self_id();
        self.publish_module_with_id(id.clone(), module)?;
        Ok(id)
    }

    /// Add module with `ModuleId` to internal state.
    pub fn publish_module_with_id(&self, id: ModuleId, module: Vec<u8>) -> Result<(), Error> {
        self.insert((&id).into(), module);
        Ok(())
    }

    /// Clear internal chain data.
    pub fn clear(&self) {
        let mut data = self.data.lock().unwrap();
        data.clear();
    }

    /// Returns chain data by access path.
    pub fn get(&self, access_path: &AccessPath) -> Option<Vec<u8>> {
        let data = &self.data.lock().unwrap();
        data.get(access_path).cloned()
    }
}

impl MockDataSource {
    /// Wrapper around internal `HashMap.insert()`.
    pub fn insert(&self, access_path: AccessPath, blob: Vec<u8>) {
        let data = &mut self.data.lock().unwrap();
        data.insert(access_path, blob);
    }

    /// Wrapper around internal `HashMap.delete()`.
    pub fn delete(&self, access_path: AccessPath) {
        let data = &mut self.data.lock().unwrap();
        data.remove(&access_path);
    }

    /// Merge `WriteSet` into internal chain state.
    pub fn merge_write_set(&self, write_set: WriteSet) {
        let data = &mut self.data.lock().unwrap();
        for (access_path, write_op) in write_set {
            match write_op {
                WriteOp::Value(blob) => {
                    data.insert(access_path.clone(), blob.clone());
                }
                WriteOp::Deletion => {
                    data.remove(&access_path);
                }
            }
        }
    }
}

impl RemoteCache for MockDataSource {
    fn get_module(&self, module_id: &ModuleId) -> VMResult<Option<Vec<u8>>> {
        Ok(self.get(&AccessPath::from(module_id)))
    }

    fn get_resource(
        &self,
        address: &AccountAddress,
        tag: &TypeTag,
    ) -> PartialVMResult<Option<Vec<u8>>> {
        let struct_tag = match tag {
            TypeTag::Struct(struct_tag) => struct_tag.clone(),
            _ => return Err(PartialVMError::new(StatusCode::VALUE_DESERIALIZATION_ERROR)),
        };
        let resource_tag = ResourceKey::new(*address, struct_tag);
        let path = AccessPath::resource_access_path(&resource_tag);
        Ok(self.get(&path))
    }
}

impl Clear for MockDataSource {}

impl DataSource for MockDataSource {}
