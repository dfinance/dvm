use std::collections::HashMap;
use std::sync::{Mutex, Arc};
use anyhow::Error;
use libra::{libra_types, libra_state_view, libra_vm, move_vm_state};
use move_vm_state::data_cache::RemoteCache;
use libra_state_view::StateView;
use libra_types::access_path::AccessPath;
use libra_types::write_set::{WriteSet, WriteOp, WriteSetMut};
use libra_vm::errors::VMResult;
use crate::{MergeWriteSet, DataSource, Clear};
use libra_vm::CompiledModule;
use libra_types::language_storage::ModuleId;

#[derive(Debug, Clone, Default)]
pub struct MockDataSource {
    data: Arc<Mutex<HashMap<AccessPath, Vec<u8>>>>,
}

impl MockDataSource {
    pub fn new() -> MockDataSource {
        MockDataSource {
            data: Arc::new(Mutex::new(Default::default())),
        }
    }

    pub fn with_write_set(write_set: WriteSet) -> MockDataSource {
        let ds = MockDataSource::new();
        ds.merge_write_set(write_set);
        ds
    }

    pub fn to_write_set(&self) -> Result<WriteSet, Error> {
        let data = self.data.lock().unwrap();
        let ws = data
            .iter()
            .map(|(path, blob)| (path.clone(), WriteOp::Value(blob.clone())))
            .collect();
        WriteSetMut::new(ws).freeze()
    }

    pub fn publish_module(&self, module: Vec<u8>) -> Result<ModuleId, Error> {
        let id = CompiledModule::deserialize(&module)?.self_id();
        self.publish_module_with_id(id.clone(), module)?;
        Ok(id)
    }

    pub fn publish_module_with_id(&self, id: ModuleId, module: Vec<u8>) -> Result<(), Error> {
        self.insert((&id).into(), module);
        Ok(())
    }

    pub fn clear(&self) {
        let mut data = self.data.lock().unwrap();
        data.clear();
    }
}

impl StateView for MockDataSource {
    fn get(&self, access_path: &AccessPath) -> Result<Option<Vec<u8>>, Error> {
        let data = &self.data.lock().unwrap();
        Ok(data.get(access_path).cloned())
    }

    // Function not currently in use.
    fn multi_get(&self, access_paths: &[AccessPath]) -> Result<Vec<Option<Vec<u8>>>, Error> {
        let data = &self.data.lock().unwrap();
        access_paths
            .iter()
            .map(|path| Ok(data.get(path).cloned()))
            .collect()
    }

    fn is_genesis(&self) -> bool {
        // It doesn’t matter since we do not have a blockchain.
        false
    }
}

impl MockDataSource {
    pub fn insert(&self, access_path: AccessPath, blob: Vec<u8>) {
        let data = &mut self.data.lock().unwrap();
        data.insert(access_path, blob);
    }

    pub fn delete(&self, access_path: AccessPath) {
        let data = &mut self.data.lock().unwrap();
        data.remove(&access_path);
    }
}

impl MergeWriteSet for MockDataSource {
    fn merge_write_set(&self, write_set: WriteSet) {
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
    fn get(&self, access_path: &AccessPath) -> VMResult<Option<Vec<u8>>> {
        Ok(StateView::get(self, access_path).unwrap())
    }
}

impl Clear for MockDataSource {
    fn clear(&self) {
        let data = &mut self.data.lock().unwrap();
        data.clear();
    }
}

impl DataSource for MockDataSource {}
