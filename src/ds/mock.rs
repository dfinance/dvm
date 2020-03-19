use std::collections::HashMap;
use std::sync::Mutex;
use anyhow::Error;
use dvm_api::tonic;
use tonic::codegen::Arc;
use libra::{libra_types, libra_state_view, vm, vm_runtime};
use libra_state_view::StateView;
use libra_types::access_path::AccessPath;
use libra_types::write_set::{WriteSet, WriteOp};
use crate::ds::MergeWriteSet;
use vm_runtime::data_cache::RemoteCache;
use vm::errors::VMResult;
use crate::vm::stdlib::{Stdlib, build_std, move_std, mvir_std};
use crate::vm::compiler::Lang;

#[derive(Debug, Clone)]
pub struct MockDataSource {
    data: Arc<Mutex<HashMap<AccessPath, Vec<u8>>>>,
}

impl MockDataSource {
    pub fn new(lang: Lang) -> MockDataSource {
        let ds = MockDataSource {
            data: Arc::new(Mutex::new(Default::default())),
        };

        let std = match &lang {
            Lang::Move => move_std(),
            Lang::MvIr => mvir_std(),
        };

        let ws = build_std(Stdlib { modules: std, lang }).unwrap();
        ds.merge_write_set(ws);
        ds
    }

    pub fn without_std() -> MockDataSource {
        MockDataSource {
            data: Arc::new(Mutex::new(Default::default())),
        }
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
