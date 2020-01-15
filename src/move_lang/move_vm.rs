extern crate lazy_static;

use lazy_static::lazy_static;

use libra_state_view::StateView;
use libra_types::transaction::{TransactionArgument, TransactionStatus};
use libra_types::{
    account_address::AccountAddress,
    transaction::{Module, Script},
};
use stdlib::stdlib_modules;
use vm::{
    gas_schedule::{CostTable, GasUnits, GasAlgebra, GasPrice},
    transaction_metadata::TransactionMetadata,
    CompiledModule,
};
use vm_cache_map::Arena;
use vm_runtime::{
    chain_state::TransactionExecutionContext, data_cache::BlockDataCache,
    execution_context::InterpreterContext, loaded_data::loaded_module::LoadedModule,
    runtime::VMRuntime, TXN_TOTAL_GAS_USAGE, VM_COUNTERS,
};
use vm_runtime_types::value::Value;
use std::fmt;
use libra_types::language_storage::ModuleId;
use libra_types::identifier::IdentStr;
use libra_types::vm_error::{VMStatus, StatusCode};
use vm::errors::{vm_error, Location, VMResult};
use crate::move_lang::gas_schedule::cost_table;
use libra_types::write_set::WriteSet;
use libra_types::contract_event::ContractEvent;

lazy_static! {
    static ref ALLOCATOR: Arena<LoadedModule> = Arena::new();
}

fn allocator() -> &'static Arena<LoadedModule> {
    &*ALLOCATOR
}

#[derive(Debug)]
pub struct ExecutionMeta {
    max_gas_amount: u64,
    gas_unit_price: u64,
    sender: AccountAddress,
}

impl ExecutionMeta {
    pub fn new(max_gas_amount: u64, gas_unit_price: u64, sender: AccountAddress) -> ExecutionMeta {
        ExecutionMeta {
            max_gas_amount,
            gas_unit_price,
            sender,
        }
    }

    pub fn test() -> ExecutionMeta {
        ExecutionMeta {
            max_gas_amount: 1_000_000,
            gas_unit_price: 1,
            sender: Default::default(),
        }
    }
}

impl Into<TransactionMetadata> for ExecutionMeta {
    fn into(self) -> TransactionMetadata {
        let mut tx_meta = TransactionMetadata::default();
        tx_meta.sender = self.sender;
        tx_meta.max_gas_amount = GasUnits::new(self.max_gas_amount);
        tx_meta.gas_unit_price = GasPrice::new(self.gas_unit_price);
        tx_meta
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionResult {
    pub write_set: WriteSet,
    pub events: Vec<ContractEvent>,
    pub gas_used: u64,
    pub status: TransactionStatus,
}

impl ExecutionResult {
    fn new(
        mut context: TransactionExecutionContext,
        txn_data: TransactionMetadata,
        result: VMResult<()>,
    ) -> VmResult {
        let gas_used: u64 = txn_data
            .max_gas_amount()
            .sub(context.gas_left())
            .mul(txn_data.gas_unit_price())
            .get();

        let write_set = context.make_write_set()?;
        record_stats!(observe | TXN_TOTAL_GAS_USAGE | gas_used);

        Ok(ExecutionResult {
            write_set,
            events: context.events().to_vec(),
            gas_used,
            status: match result {
                Ok(()) => TransactionStatus::from(VMStatus::new(StatusCode::EXECUTED)),
                Err(err) => TransactionStatus::from(err),
            },
        })
    }
}

pub type VmResult = Result<ExecutionResult, VMStatus>;

// XXX: not used currently
pub trait VM {
    fn create_account(&self, meta: ExecutionMeta, address: AccountAddress) -> VmResult;
    fn publish_module(&self, meta: ExecutionMeta, module: Module) -> VmResult;
    fn execute_script(&self, meta: ExecutionMeta, script: Script) -> VmResult;
    fn execute_function(
        &self,
        meta: ExecutionMeta,
        module_id: &ModuleId,
        function_name: &IdentStr,
        args: Vec<TransactionArgument>,
    ) -> VmResult;
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
            cost_table: cost_table(),
        }
    }

    fn make_data_cache(&self) -> BlockDataCache {
        BlockDataCache::new(self.view.as_ref())
    }

    fn make_execution_context<'a>(
        &self,
        meta: &TransactionMetadata,
        cache: &'a BlockDataCache,
    ) -> TransactionExecutionContext<'a> {
        TransactionExecutionContext::new(meta.max_gas_amount, cache)
    }
}

impl fmt::Debug for MoveVm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MoveVm {{ }}")
    }
}

impl VM for MoveVm {
    fn create_account(&self, meta: ExecutionMeta, address: AccountAddress) -> VmResult {
        let cache = self.make_data_cache();
        let meta = meta.into();

        let mut context = self.make_execution_context(&meta, &cache);
        let res = self
            .runtime
            .create_account(&mut context, &meta, &self.cost_table, address);

        ExecutionResult::new(context, meta, res)
    }

    fn publish_module(&self, meta: ExecutionMeta, module: Module) -> VmResult {
        let cache = self.make_data_cache();
        let meta = meta.into();
        let mut context = self.make_execution_context(&meta, &cache);

        let module = module.into_inner();
        let res = CompiledModule::deserialize(&module).and_then(|compiled_module| {
            let module_id = compiled_module.self_id();
            if InterpreterContext::exists_module(&context, &module_id) {
                return Err(vm_error(
                    Location::default(),
                    StatusCode::DUPLICATE_MODULE_NAME,
                ));
            }

            InterpreterContext::publish_module(&mut context, module_id, module)
        });

        ExecutionResult::new(context, meta, res)
    }

    fn execute_script(&self, meta: ExecutionMeta, script: Script) -> VmResult {
        let cache = self.make_data_cache();
        let meta = meta.into();

        let mut context = self.make_execution_context(&meta, &cache);

        let (script, args) = script.into_inner();

        let res = convert_txn_args(args).and_then(|args| {
            self.runtime
                .execute_script(&mut context, &meta, &self.cost_table, script, args)
        });

        ExecutionResult::new(context, meta, res)
    }

    fn execute_function(
        &self,
        meta: ExecutionMeta,
        module_id: &ModuleId,
        function_name: &IdentStr,
        args: Vec<TransactionArgument>,
    ) -> VmResult {
        let cache = self.make_data_cache();
        let meta = meta.into();

        let mut context = self.make_execution_context(&meta, &cache);

        let res = convert_txn_args(args).and_then(|args| {
            self.runtime.execute_function(
                &mut context,
                &meta,
                &self.cost_table,
                module_id,
                &function_name,
                args,
            )
        });

        ExecutionResult::new(context, meta, res)
    }
}

/// Convert the transaction arguments into move values.
fn convert_txn_args(args: Vec<TransactionArgument>) -> Result<Vec<Value>, VMStatus> {
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
    use crate::move_lang::move_vm::ExecutionMeta;
    use libra_types::vm_error::StatusCode::DUPLICATE_MODULE_NAME;

    #[test]
    fn test_create_account() {
        let ds = MockDataSource::default();
        let vm = MoveVm::new(Box::new(ds.clone()));
        let account = AccountAddress::random();
        assert!(ds.get_account(&account).unwrap().is_none());
        let output = vm.create_account(ExecutionMeta::test(), account).unwrap();
        ds.merge_write_set(output.write_set).unwrap();
        assert_eq!(ds.get_account(&account).unwrap().unwrap().balance(), 0);
    }

    #[test]
    fn test_publish_module() {
        let ds = MockDataSource::default();
        let vm = MoveVm::new(Box::new(ds.clone()));
        let account = AccountAddress::random();
        let output = vm.create_account(ExecutionMeta::test(), account).unwrap();
        ds.merge_write_set(output.write_set).unwrap();

        let program = "module M {}";
        let unit = build(Code::module("M", program), &account).unwrap();
        let module = Module::new(unit.serialize());
        let output = vm
            .publish_module(ExecutionMeta::test(), module.clone())
            .unwrap();

        let compiled_module = CompiledModule::deserialize(&module.code()).unwrap();
        let module_id = compiled_module.self_id();

        assert!(ds.get_module(&module_id).unwrap().is_none());

        ds.merge_write_set(output.write_set).unwrap();

        let loaded_module = ds.get_module(&module_id).unwrap().unwrap();
        assert_eq!(loaded_module, module);

        //try public module duplicate;
        assert_eq!(
            DUPLICATE_MODULE_NAME,
            vm.publish_module(ExecutionMeta::test(), module)
                .unwrap()
                .status
                .vm_status()
                .major_status
        );
    }

    #[test]
    fn test_execute_function() {
        let ds = MockDataSource::default();
        let vm = MoveVm::new(Box::new(ds.clone()));

        ds.merge_write_set(
            vm.create_account(ExecutionMeta::test(), association_address())
                .unwrap()
                .write_set,
        )
        .unwrap();
        ds.merge_write_set(
            vm.create_account(ExecutionMeta::test(), transaction_fee_address())
                .unwrap()
                .write_set,
        )
        .unwrap();
        ds.merge_write_set(
            vm.create_account(ExecutionMeta::test(), core_code_address())
                .unwrap()
                .write_set,
        )
        .unwrap();

        ds.merge_write_set(
            vm.execute_function(
                ExecutionMeta::new(1_000, 1, association_address()),
                &COIN_MODULE,
                &Identifier::new("initialize").unwrap(),
                vec![],
            )
            .unwrap()
            .write_set,
        )
        .unwrap();

        let account = AccountAddress::random();
        let output = vm.create_account(ExecutionMeta::test(), account).unwrap();
        ds.merge_write_set(output.write_set).unwrap();

        let output = vm
            .execute_function(
                ExecutionMeta::new(1_000, 1, association_address()),
                &ACCOUNT_MODULE,
                &Identifier::new("mint_to_address").unwrap(),
                vec![
                    TransactionArgument::Address(account),
                    TransactionArgument::U64(1000),
                ],
            )
            .unwrap();
        ds.merge_write_set(output.write_set).unwrap();
    }

    #[test]
    fn test_execute_script() {
        let ds = MockDataSource::default();
        let vm = MoveVm::new(Box::new(ds.clone()));
        ds.merge_write_set(
            vm.create_account(ExecutionMeta::test(), association_address())
                .unwrap()
                .write_set,
        )
        .unwrap();
        ds.merge_write_set(
            vm.create_account(ExecutionMeta::test(), transaction_fee_address())
                .unwrap()
                .write_set,
        )
        .unwrap();
        ds.merge_write_set(
            vm.create_account(ExecutionMeta::test(), core_code_address())
                .unwrap()
                .write_set,
        )
        .unwrap();
        ds.merge_write_set(
            vm.execute_function(
                ExecutionMeta::new(100_000, 1, association_address()),
                &COIN_MODULE,
                &Identifier::new("initialize").unwrap(),
                vec![],
            )
            .unwrap()
            .write_set,
        )
        .unwrap();

        let account = AccountAddress::random();
        let output = vm.create_account(ExecutionMeta::test(), account).unwrap();
        ds.merge_write_set(output.write_set).unwrap();

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
        let output = vm
            .execute_script(
                ExecutionMeta::new(100_000, 1, association_address()),
                script,
            )
            .unwrap();
        ds.merge_write_set(output.write_set).unwrap();
        assert!(output.gas_used > 0);
        assert_eq!(ds.get_account(&account).unwrap().unwrap().balance(), 1000);
    }
}
