use libra_types::access_path::AccessPath;
use anyhow::Error;
use std::collections::HashMap;
use libra_state_view::StateView;
use crate::ds::{MergeWriteSet, DataAccess};
use libra_types::write_set::{WriteSet, WriteOp};
use libra_types::account_address::AccountAddress;
use libra_types::account_config::AccountResource;
use vm_runtime::identifier::create_access_path;
use libra_types::account_config;
use tonic::codegen::Arc;
use std::sync::Mutex;
use libra_types::language_storage::ModuleId;
use libra_types::transaction::Module;

#[derive(Debug, Default, Clone)]
pub struct MockDataSource {
    data: Arc<Mutex<HashMap<AccessPath, Vec<u8>>>>,
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

impl MergeWriteSet for MockDataSource {
    fn merge_write_set(&mut self, write_set: &WriteSet) -> Result<(), Error> {
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
        Ok(())
    }
}

impl DataAccess for MockDataSource {
    fn get_account(&self, address: &AccountAddress) -> Result<Option<AccountResource>, Error> {
        let entry = self.get(&create_access_path(
            address,
            account_config::account_struct_tag(),
        ))?;
        Ok(entry
            .map(|data| lcs::from_bytes(data.as_slice()))
            .map_or(Ok(None), |v| v.map(Some))?)
    }

    fn get_module(&self, module_id: &ModuleId) -> Result<Option<Module>, Error> {
        let entry = self.get(&AccessPath::from(module_id))?;
        Ok(entry.map(Module::new))
    }
}
