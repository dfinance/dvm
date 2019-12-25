mod move_vm;
pub use move_vm::MoveVm;

use anyhow::Result;
use libra_types::{
    account_address::AccountAddress,
    transaction::{Module, Script},
    write_set::WriteSet,
};
use vm_cache_map::Arena;
use vm_runtime::loaded_data::loaded_module::LoadedModule;


pub trait VM {
    fn create_account(&self, address: AccountAddress) -> Result<WriteSet>;
    fn publish_module(&self, module: Module) -> Result<WriteSet>;
    fn execute_script(&self, script: Script) -> Result<WriteSet>;
}
