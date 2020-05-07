use libra::{libra_types, libra_vm, move_vm_runtime};
use libra_types::transaction::TransactionStatus;
use libra_types::{account_address::AccountAddress, transaction::Module};
use libra_vm::{transaction_metadata::TransactionMetadata, CompiledModule};
use libra::move_core_types::gas_schedule::{GasAlgebra, GasPrice, GasUnits, CostTable};
use std::fmt;
use libra_types::vm_error::{VMStatus, StatusCode};
use libra_vm::errors::{vm_error, Location, VMResult};
use libra_types::write_set::WriteSet;

use libra_types::contract_event::ContractEvent;
use libra::move_vm_state::execution_context::{ExecutionContext, TransactionExecutionContext};
use libra::move_vm_types::values::Value;
use ds::DataSource;
use libra::move_vm_state::data_cache::BlockDataCache;
use libra::move_vm_types::interpreter_context::InterpreterContext;
use move_vm_runtime::{MoveVM, loader::ModuleCache};
use anyhow::Error;
use crate::gas_schedule;
use libra_types::language_storage::TypeTag;
use serde_derive::Deserialize;
use libra_types::account_config::CORE_CODE_ADDRESS;
use std::collections::HashMap;

#[derive(Debug)]
pub struct ExecutionMeta {
    pub max_gas_amount: u64,
    pub gas_unit_price: u64,
    pub sender: AccountAddress,
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
            .sub(context.remaining_gas())
            .mul(txn_data.gas_unit_price())
            .get();

        Ok(ExecutionResult {
            write_set: context.make_write_set()?,
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
    fn publish_module(&self, meta: ExecutionMeta, module: Module) -> VmResult;
    fn execute_script(&self, meta: ExecutionMeta, script: Script) -> VmResult;
}

pub struct Dvm<D: DataSource> {
    vm: MoveVM,
    ds: D,
    cost_table: CostTable,
}

impl<D> Dvm<D>
where
    D: DataSource,
{
    pub fn new(ds: D) -> Result<Dvm<D>, Error> {
        let vm = MoveVM::new();

        trace!("vm service is ready.");
        Ok(Dvm {
            vm,
            ds,
            cost_table: gas_schedule::cost_table(),
        })
    }

    fn make_data_cache(&self) -> BlockDataCache {
        BlockDataCache::new(&self.ds)
    }

    fn make_execution_context<'a>(
        &self,
        meta: &TransactionMetadata,
        cache: &'a BlockDataCache,
    ) -> TransactionExecutionContext<'a> {
        TransactionExecutionContext::new(meta.max_gas_amount, cache)
    }
}

impl<D> fmt::Debug for Dvm<D>
where
    D: DataSource,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Dvm {{ }}")
    }
}

impl<D> VM for Dvm<D>
where
    D: DataSource,
{
    fn publish_module(&self, meta: ExecutionMeta, module: Module) -> VmResult {
        let cache = self.make_data_cache();
        let meta = meta.into();
        let mut context = self.make_execution_context(&meta, &cache);

        let module = module.into_inner();
        let res = CompiledModule::deserialize(&module).and_then(|compiled_module| {
            let module_id = compiled_module.self_id();
            if meta.sender == CORE_CODE_ADDRESS && *module_id.address() == CORE_CODE_ADDRESS {
                self.ds.clear();
                let loader = &self.vm.runtime.loader;
                *loader.libra_cache.lock().unwrap() = HashMap::new();
                *loader.module_cache.lock().unwrap() = ModuleCache::new();
            } else {
                if InterpreterContext::exists_module(&context, &module_id) {
                    return Err(vm_error(
                        Location::default(),
                        StatusCode::DUPLICATE_MODULE_NAME,
                    ));
                }
            }
            InterpreterContext::publish_module(&mut context, module_id, module)
        });

        ExecutionResult::new(context, meta, res)
    }

    fn execute_script(&self, meta: ExecutionMeta, script: Script) -> VmResult {
        let cache = self.make_data_cache();
        let meta = meta.into();
        let mut context = self.make_execution_context(&meta, &cache);

        let (script, args, type_args) = script.into_inner();
        let res = self.vm.execute_script(
            script,
            &self.cost_table,
            &mut context,
            &meta,
            type_args,
            args,
        );
        ExecutionResult::new(context, meta, res)
    }
}

pub struct Script {
    code: Vec<u8>,
    args: Vec<Value>,
    type_args: Vec<TypeTag>,
}

impl Script {
    pub fn new(code: Vec<u8>, args: Vec<Value>, type_args: Vec<TypeTag>) -> Self {
        Script {
            code,
            args,
            type_args,
        }
    }

    pub fn code(&self) -> &[u8] {
        &self.code
    }

    pub fn args(&self) -> &[Value] {
        &self.args
    }

    pub fn into_inner(self) -> (Vec<u8>, Vec<Value>, Vec<TypeTag>) {
        (self.code, self.args, self.type_args)
    }
}

impl fmt::Debug for Script {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Script")
            .field("code", &hex::encode(&self.code))
            .field("args", &self.args)
            .finish()
    }
}

#[derive(Deserialize, Debug, PartialEq, Eq)]
pub struct U64Store {
    pub val: u64,
}

#[derive(Deserialize, Debug, PartialEq, Eq)]
pub struct AddressStore {
    pub val: AccountAddress,
}

#[derive(Deserialize, Debug, PartialEq, Eq)]
pub struct VectorU8Store {
    pub val: Vec<u8>,
}

#[cfg(test)]
pub mod tests {
    use lang::{compiler::Compiler, stdlib::zero_sdt};
    use libra::{
        libra_types::{
            account_address::AccountAddress, transaction::Module, vm_error::StatusCode,
            write_set::WriteOp,
        },
        libra_vm::CompiledModule,
        lcs,
    };
    use ds::{MockDataSource, MergeWriteSet, DataAccess};
    use libra::move_vm_types::values::Value;
    use crate::move_vm::{ExecutionMeta, Dvm, VM, Script, U64Store};

    #[test]
    fn test_publish_module() {
        let ds = MockDataSource::with_write_set(zero_sdt());
        let compiler = Compiler::new(ds.clone());
        let vm = Dvm::new(ds.clone()).unwrap();
        let account = AccountAddress::random();

        let program = "module M {}";
        let module = Module::new(compiler.compile(program, &account).unwrap());
        let output = vm
            .publish_module(ExecutionMeta::test(), module.clone())
            .unwrap();

        let compiled_module = CompiledModule::deserialize(&module.code()).unwrap();
        let module_id = compiled_module.self_id();

        assert!(ds.get_module(&module_id).unwrap().is_none());

        ds.merge_write_set(output.write_set);

        let loaded_module = ds.get_module(&module_id).unwrap().unwrap();
        assert_eq!(loaded_module, module);

        //try public module duplicate;
        assert_eq!(
            StatusCode::DUPLICATE_MODULE_NAME,
            vm.publish_module(ExecutionMeta::test(), module)
                .unwrap()
                .status
                .vm_status()
                .major_status
        );
    }

    #[test]
    fn test_execute_script() {
        let ds = MockDataSource::with_write_set(zero_sdt());
        let compiler = Compiler::new(ds.clone());
        let vm = Dvm::new(ds.clone()).unwrap();
        let account = AccountAddress::random();

        let module = include_str!("../../test-kit/tests/resources/store.move");
        let module = Module::new(compiler.compile(module, &account).unwrap());
        ds.merge_write_set(
            vm.publish_module(ExecutionMeta::test(), module)
                .unwrap()
                .write_set,
        );

        let script = format!(
            "
            use 0x{}::Store;
            fun main(val: u64) {{
                Store::store_u64(val);
            }}
        ",
            account
        );
        let script = compiler.compile(&script, &account).unwrap();
        let test_value = U64Store { val: 100 };
        let result = vm
            .execute_script(
                ExecutionMeta::test(),
                Script::new(script, vec![Value::u64(test_value.val)], vec![]),
            )
            .unwrap();
        assert!(!result.write_set.is_empty());
        let (_, op) = result.write_set.iter().next().unwrap();
        if let WriteOp::Value(blob) = op {
            let value_store: U64Store = lcs::from_bytes(&blob).unwrap();
            assert_eq!(test_value, value_store);
        } else {
            unreachable!();
        }
    }
}
