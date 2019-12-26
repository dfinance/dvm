extern crate lazy_static;
use lazy_static::lazy_static;

use crate::vm::VM;
use anyhow::{Error, Result};
use libra_config::config::{VMConfig, VMPublishingOption};
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

lazy_static! {
    static ref ALLOCATOR: Arena<LoadedModule> = Arena::new();
}

fn allocator() -> &'static Arena<LoadedModule> {
    &*ALLOCATOR
}

pub struct MoveVm {
    runtime: VMRuntime<'static>,
    view: Box<dyn StateView>,
    cost_table: CostTable,
}

impl MoveVm {
    pub fn new(view: Box<dyn StateView>) -> MoveVm {
        let config = VMConfig {
            publishing_options: VMPublishingOption::Open,
        };

        let mut runtime = VMRuntime::new(&allocator(), &config);

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

impl VM for MoveVm {
    fn create_account(&self, address: AccountAddress) -> Result<WriteSet> {
        let cache = self.make_data_cache();
        let mut context = self.make_execution_context(&cache);

        self.runtime.create_account(
            self.view.as_ref(),
            &mut context,
            &TransactionMetadata::default(),
            &self.cost_table,
            address,
        )?;
        Ok(context.make_write_set(vec![])?)
    }

    fn publish_module(&self, module: Module) -> Result<WriteSet> {
        let cache = self.make_data_cache();
        let mut context = self.make_execution_context(&cache);

        let module = module.into_inner();
        let compiled_module = CompiledModule::deserialize(&module)?;
        let module_id = compiled_module.self_id();

        if context.exists_module(&module_id) {
            return Err(Error::msg("Duplicate module name"));
        }

        Ok(context.make_write_set(vec![(module_id, module)])?)
    }

    fn execute_script(&self, script: Script) -> Result<WriteSet> {
        let cache = self.make_data_cache();
        let mut context = self.make_execution_context(&cache);

        let (script, args) = script.into_inner();
        self.runtime.execute_script(
            self.view.as_ref(),
            &mut context,
            &TransactionMetadata::default(),
            &self.cost_table,
            script,
            convert_txn_args(args)?,
        )?;

        Ok(context.make_write_set(vec![])?)
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
