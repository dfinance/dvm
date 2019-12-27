use libra_types::access_path::AccessPath;
use anyhow::Error;
use std::collections::HashMap;
use libra_state_view::StateView;

#[derive(Debug, Default)]
pub struct MockDataSource {
    data: HashMap<AccessPath, Vec<u8>>,
}

impl StateView for MockDataSource {
    fn get(&self, access_path: &AccessPath) -> Result<Option<Vec<u8>>, Error> {
        Ok(self.data.get(access_path).cloned())
    }

    // Function not currently in use.
    fn multi_get(&self, access_paths: &[AccessPath]) -> Result<Vec<Option<Vec<u8>>>, Error> {
        access_paths.iter().map(|path| self.get(path)).collect()
    }

    fn is_genesis(&self) -> bool {
        // It doesn’t matter since we do not have a blockchain.
        false
    }
}



