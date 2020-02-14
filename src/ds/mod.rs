pub mod mock;
pub mod view;

use anyhow::Error;
use libra_types::write_set::WriteSet;

pub use mock::MockDataSource;

use libra_types::account_address::AccountAddress;
use libra_types::account_config::AccountResource;
use libra_types::account_config::account_struct_tag;
use libra_types::language_storage::ModuleId;
use libra_types::transaction::Module;
use libra_types::access_path::AccessPath;
use libra_state_view::StateView;
use vm_runtime::identifier::create_access_path;

pub trait MergeWriteSet {
    fn merge_write_set(&self, write_set: WriteSet);
}

pub trait DataAccess {
    fn get_account(&self, address: &AccountAddress) -> Result<Option<AccountResource>, Error>;
    fn get_module(&self, module_id: &ModuleId) -> Result<Option<Module>, Error>;
}

// auto-impl for all StateView impls
impl<T: StateView> DataAccess for T {
    fn get_account(&self, address: &AccountAddress) -> Result<Option<AccountResource>, Error> {
        let entry = self.get(&create_access_path(address, account_struct_tag()))?;
        Ok(entry
            .map(|data| lcs::from_bytes(data.as_slice()))
            .map_or(Ok(None), |v| v.map(Some))?)
    }

    fn get_module(&self, module_id: &ModuleId) -> Result<Option<Module>, Error> {
        let entry = self.get(&AccessPath::from(module_id))?;
        Ok(entry.map(Module::new))
    }
}
