mod grpc;
mod mock;

use anyhow::Error;
use libra_types::write_set::WriteSet;

pub use mock::MockDataSource;
use libra_types::account_address::AccountAddress;
use libra_types::account_config::AccountResource;
use libra_types::language_storage::ModuleId;
use libra_types::transaction::Module;

pub trait MergeWriteSet {
    fn merge_write_set(&mut self, write_set: WriteSet) -> Result<(), Error>;
}

pub trait DataAccess {
    fn get_account(&self, address: &AccountAddress) -> Result<Option<AccountResource>, Error>;
    fn get_module(&self, module_id: &ModuleId) -> Result<Option<Module>, Error>;
}
