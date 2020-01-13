extern crate lazy_static;

use lazy_static::lazy_static;

use anyhow::{Error, Result};
use libra_state_view::StateView;
use libra_types::transaction::TransactionArgument;
use libra_types::{
    account_address::AccountAddress,
    transaction::{Module, Script},
    write_set::WriteSet,
};
use stdlib::stdlib_modules;
use vm::{
    gas_schedule::{CostTable, MAXIMUM_NUMBER_OF_GAS_UNITS},
    transaction_metadata::TransactionMetadata,
    CompiledModule,
};
use vm_cache_map::Arena;
use vm_runtime::{
    chain_state::TransactionExecutionContext, data_cache::BlockDataCache,
    execution_context::InterpreterContext, loaded_data::loaded_module::LoadedModule,
    runtime::VMRuntime,
};
use vm_runtime_types::value::Value;
use std::fmt;
use libra_types::language_storage::ModuleId;
use libra_types::identifier::IdentStr;

lazy_static! {
    static ref ALLOCATOR: Arena<LoadedModule> = Arena::new();
}

fn allocator() -> &'static Arena<LoadedModule> {
    &*ALLOCATOR
}

// XXX: not used currently
pub trait VM {
    fn create_account(&self, address: AccountAddress) -> Result<WriteSet>;
    fn publish_module(&self, module: Module) -> Result<WriteSet>;
    fn execute_script(&self, executor: AccountAddress, script: Script) -> Result<WriteSet>;
    fn execute_function(
        &self,
        executor: AccountAddress,
        module_id: &ModuleId,
        function_name: &IdentStr,
        args: Vec<TransactionArgument>,
    ) -> Result<WriteSet>;
}

pub struct MoveVm {
    runtime: VMRuntime<'static>,
    view: Box<dyn StateView>,
    cost_table: CostTable,
}

impl MoveVm {
    pub fn new(view: Box<dyn StateView>) -> MoveVm {
        let mut runtime = VMRuntime::new(allocator());

        let modules = stdlib_modules();
        for module in modules {
            runtime.cache_module(module.clone());
        }

        MoveVm {
            runtime,
            view,
            cost_table: CostTable::zero(),
        }
    }

    fn make_data_cache(&self) -> BlockDataCache {
        BlockDataCache::new(self.view.as_ref())
    }

    fn make_execution_context<'a>(
        &self,
        cache: &'a BlockDataCache,
    ) -> TransactionExecutionContext<'a> {
        TransactionExecutionContext::new(*MAXIMUM_NUMBER_OF_GAS_UNITS, cache)
    }
}

impl fmt::Debug for MoveVm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MoveVm {{ }}")
    }
}

impl VM for MoveVm {
    fn create_account(&self, address: AccountAddress) -> Result<WriteSet> {
        let cache = self.make_data_cache();
        let mut context = self.make_execution_context(&cache);

        self.runtime.create_account(
            &mut context,
            &TransactionMetadata::default(),
            &self.cost_table,
            address,
        )?;
        Ok(context.make_write_set()?)
    }

    fn publish_module(&self, module: Module) -> Result<WriteSet> {
        let cache = self.make_data_cache();
        let mut context = self.make_execution_context(&cache);

        let module = module.into_inner();
        let compiled_module = CompiledModule::deserialize(&module)?;
        let module_id = compiled_module.self_id();

        if InterpreterContext::exists_module(&context, &module_id) {
            return Err(Error::msg("Duplicate module name"));
        }

        InterpreterContext::publish_module(&mut context, module_id, module)?;
        Ok(context.make_write_set()?)
    }

    fn execute_script(&self, executor: AccountAddress, script: Script) -> Result<WriteSet> {
        let cache = self.make_data_cache();
        let mut context = self.make_execution_context(&cache);

        let (script, args) = script.into_inner();
        let mut meta = TransactionMetadata::default();
        meta.sender = executor;

        self.runtime.execute_script(
            &mut context,
            &meta,
            &self.cost_table,
            script,
            convert_txn_args(args)?,
        )?;

        Ok(context.make_write_set()?)
    }

    fn execute_function(
        &self,
        executor: AccountAddress,
        module_id: &ModuleId,
        function_name: &IdentStr,
        args: Vec<TransactionArgument>,
    ) -> Result<WriteSet, Error> {
        let cache = self.make_data_cache();
        let mut context = self.make_execution_context(&cache);

        let mut meta = TransactionMetadata::default();
        meta.sender = executor;

        self.runtime.execute_function(
            &mut context,
            &meta,
            &self.cost_table,
            module_id,
            &function_name,
            convert_txn_args(args)?,
        )?;
        Ok(context.make_write_set()?)
    }
}

/// Convert the transaction arguments into move values.
fn convert_txn_args(args: Vec<TransactionArgument>) -> Result<Vec<Value>> {
    args.into_iter()
        .map(|arg| match arg {
            TransactionArgument::U64(i) => Ok(Value::u64(i)),
            TransactionArgument::Address(a) => Ok(Value::address(a)),
            TransactionArgument::Bool(b) => Ok(Value::bool(b)),
            TransactionArgument::ByteArray(b) => Ok(Value::byte_array(b)),
        })
        .collect()
}

#[cfg(test)]
mod test {
    use crate::move_lang::{MoveVm, VM, build};
    use libra_types::account_address::AccountAddress;
    use crate::ds::{MockDataSource, MergeWriteSet, DataAccess};
    use libra_types::transaction::{Module, Script, TransactionArgument};
    use vm::CompiledModule;
    use vm_runtime::system_module_names::{ACCOUNT_MODULE, COIN_MODULE};
    use libra_types::identifier::Identifier;
    use libra_types::account_config::{core_code_address, association_address, transaction_fee_address};
    use crate::move_lang::compiler::Code;

    #[test]
    fn test_create_account() {
        let mut ds = MockDataSource::default();
        let vm = MoveVm::new(Box::new(ds.clone()));
        let account = AccountAddress::random();
        assert!(ds.get_account(&account).unwrap().is_none());
        let merge_set = vm.create_account(account).unwrap();
        ds.merge_write_set(merge_set).unwrap();
        assert_eq!(ds.get_account(&account).unwrap().unwrap().balance(), 0);
    }

    #[test]
    fn test_publish_module() {
        let mut ds = MockDataSource::default();
        let vm = MoveVm::new(Box::new(ds.clone()));
        let account = AccountAddress::random();
        let merge_set = vm.create_account(account).unwrap();
        ds.merge_write_set(merge_set).unwrap();

        let program = "module M {}";
        let unit = build(Code::module("M", program), &account).unwrap();
        let module = Module::new(unit.serialize());
        let merge_set = vm.publish_module(module.clone()).unwrap();

        let compiled_module = CompiledModule::deserialize(&module.code()).unwrap();
        let module_id = compiled_module.self_id();

        assert!(ds.get_module(&module_id).unwrap().is_none());

        ds.merge_write_set(merge_set).unwrap();

        let loaded_module = ds.get_module(&module_id).unwrap().unwrap();
        assert_eq!(loaded_module, module);

        //try public module duplicate;
        assert_eq!(
            "Duplicate module name",
            format!("{}", vm.publish_module(module).err().unwrap())
        );
    }

    #[test]
    fn test_execute_function() {
        let mut ds = MockDataSource::default();
        let vm = MoveVm::new(Box::new(ds.clone()));

        ds.merge_write_set(vm.create_account(association_address()).unwrap())
            .unwrap();
        ds.merge_write_set(vm.create_account(transaction_fee_address()).unwrap())
            .unwrap();
        ds.merge_write_set(vm.create_account(core_code_address()).unwrap())
            .unwrap();
        ds.merge_write_set(
            vm.execute_function(
                association_address(),
                &COIN_MODULE,
                &Identifier::new("initialize").unwrap(),
                vec![],
            )
            .unwrap(),
        )
        .unwrap();

        let account = AccountAddress::random();
        let merge_set = vm.create_account(account).unwrap();
        ds.merge_write_set(merge_set).unwrap();

        let merge_set = vm
            .execute_function(
                association_address(),
                &ACCOUNT_MODULE,
                &Identifier::new("mint_to_address").unwrap(),
                vec![
                    TransactionArgument::Address(account),
                    TransactionArgument::U64(1000),
                ],
            )
            .unwrap();
        ds.merge_write_set(merge_set).unwrap();
    }

    #[test]
    fn test_execute_script() {
        let mut ds = MockDataSource::default();
        let vm = MoveVm::new(Box::new(ds.clone()));
        ds.merge_write_set(vm.create_account(association_address()).unwrap())
            .unwrap();
        ds.merge_write_set(vm.create_account(transaction_fee_address()).unwrap())
            .unwrap();
        ds.merge_write_set(vm.create_account(core_code_address()).unwrap())
            .unwrap();
        ds.merge_write_set(
            vm.execute_function(
                association_address(),
                &COIN_MODULE,
                &Identifier::new("initialize").unwrap(),
                vec![],
            )
            .unwrap(),
        )
        .unwrap();

        let account = AccountAddress::random();
        let merge_set = vm.create_account(account).unwrap();
        ds.merge_write_set(merge_set).unwrap();

        let program = "
        main(payee: address, amount: u64) {
            0x0::LibraAccount::mint_to_address(payee, amount)
        }
        ";
        let unit = build(Code::script(program), &account).unwrap();
        let script = Script::new(
            unit.serialize(),
            vec![
                TransactionArgument::Address(account),
                TransactionArgument::U64(1000),
            ],
        );
        let merge_set = vm.execute_script(association_address(), script).unwrap();
        ds.merge_write_set(merge_set).unwrap();
        assert_eq!(ds.get_account(&account).unwrap().unwrap().balance(), 1000);
    }
}
