use libra::{libra_types, libra_vm, move_vm_runtime, move_vm_types};
use libra_types::transaction::TransactionStatus;
use libra_types::{account_address::AccountAddress, transaction::Module};
use libra_vm::CompiledModule;
use libra::move_core_types::gas_schedule::{GasAlgebra, GasUnits, CostTable};
use std::fmt;
use libra_types::vm_error::{VMStatus, StatusCode};
use libra_vm::errors::{vm_error, Location, VMResult};
use libra_types::write_set::WriteSet;

use libra_types::contract_event::ContractEvent;
use libra::move_vm_types::values::Value;
use ds::DataSource;
use move_vm_runtime::{loader::ModuleCache};
use crate::gas_schedule;
use libra::move_core_types::language_storage::TypeTag;
use serde_derive::Deserialize;
use libra_types::account_config::CORE_CODE_ADDRESS;
use std::collections::HashMap;
use move_vm_runtime::loader::ScriptCache;
use move_vm_runtime::move_vm::MoveVM;
use move_vm_runtime::data_cache::TransactionDataCache;
use move_vm_types::gas_schedule::CostStrategy;
use move_vm_types::data_store::DataStore;

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
            sender: CORE_CODE_ADDRESS,
        }
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
        mut data_cache: TransactionDataCache,
        cost_strategy: CostStrategy,
        txn_data: ExecutionMeta,
        result: VMResult<()>,
    ) -> VmResult {
        let gas_used = GasUnits::new(txn_data.max_gas_amount)
            .sub(cost_strategy.remaining_gas())
            .get();

        Ok(ExecutionResult {
            write_set: data_cache.make_write_set()?,
            events: data_cache.event_data().to_vec(),
            gas_used,
            status: match result {
                Ok(()) => TransactionStatus::from(VMStatus::new(StatusCode::EXECUTED)),
                Err(err) => TransactionStatus::from(err),
            },
        })
    }
}

pub type VmResult = Result<ExecutionResult, VMStatus>;

/// Dfinance virtual machine.
pub struct Dvm<D: DataSource> {
    /// Libra virtual machine.
    vm: MoveVM,
    /// Data source.
    ds: D,
    /// Instructions cost table.
    cost_table: CostTable,
}

impl<D> Dvm<D>
where
    D: DataSource,
{
    /// Create a new virtual machine with the given data source.
    pub fn new(ds: D) -> Dvm<D> {
        let vm = MoveVM::new();

        trace!("vm service is ready.");
        Dvm {
            vm,
            ds,
            cost_table: gas_schedule::cost_table(),
        }
    }

    /// Creates cache for script execution.
    fn make_data_cache(&self) -> TransactionDataCache {
        TransactionDataCache::new(&self.ds)
    }

    pub fn publish_module(&self, meta: ExecutionMeta, module: Module) -> VmResult {
        let mut cache = self.make_data_cache();
        let cost_strategy =
            CostStrategy::transaction(&self.cost_table, GasUnits::new(meta.max_gas_amount));

        let res = CompiledModule::deserialize(module.code()).and_then(|compiled_module| {
            let module_id = compiled_module.self_id();
            if meta.sender != *module_id.address() {
                return Err(vm_error(
                    Location::default(),
                    StatusCode::MODULE_ADDRESS_DOES_NOT_MATCH_SENDER,
                ));
            }

            if meta.sender == CORE_CODE_ADDRESS {
                self.ds.clear();
                let loader = &self.vm.runtime.loader;
                *loader.scripts.lock().unwrap() = ScriptCache::new();
                *loader.libra_cache.lock().unwrap() = HashMap::new();
                *loader.module_cache.lock().unwrap() = ModuleCache::new();
            } else if cache.exists_module(&module_id) {
                return Err(vm_error(
                    Location::default(),
                    StatusCode::DUPLICATE_MODULE_NAME,
                ));
            }
            cache.publish_module(module_id, module.code)
        });

        ExecutionResult::new(cache, cost_strategy, meta, res)
    }

    pub fn execute_script(&self, meta: ExecutionMeta, script: Script) -> VmResult {
        let mut cache = self.make_data_cache();

        let (script, args, type_args) = script.into_inner();
        let mut cost_strategy =
            CostStrategy::transaction(&self.cost_table, GasUnits::new(meta.max_gas_amount));

        let res = self.vm.execute_script(
            script,
            type_args,
            args,
            meta.sender,
            &mut cache,
            &mut cost_strategy,
        );
        ExecutionResult::new(cache, cost_strategy, meta, res)
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
    use compiler::Compiler;
    use lang::{stdlib::zero_sdt};
    use libra::{
        libra_types::{
            account_address::AccountAddress, transaction::Module, vm_error::StatusCode,
            write_set::WriteOp,
        },
        lcs,
    };
    use ds::{MockDataSource, MergeWriteSet, DataAccess};
    use libra::move_vm_types::values::Value;
    use crate::move_vm::{ExecutionMeta, Dvm, Script, U64Store};
    use libra::libra_vm::CompiledModule;

    #[test]
    fn test_publish_module() {
        let ds = MockDataSource::with_write_set(zero_sdt());
        let compiler = Compiler::new(ds.clone());
        let vm = Dvm::new(ds.clone());
        let account = AccountAddress::random();

        let program = "module M {}";
        let module = Module::new(compiler.compile(program, Some(account)).unwrap());
        let output = vm
            .publish_module(ExecutionMeta::new(1_000_000, 1, account), module.clone())
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
            vm.publish_module(ExecutionMeta::new(1_000_000, 1, account), module)
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
        let vm = Dvm::new(ds.clone());
        let account = AccountAddress::random();

        let module = include_str!("../../test-kit/tests/resources/store.move");
        let module = Module::new(compiler.compile(module, Some(account)).unwrap());
        ds.merge_write_set(
            vm.publish_module(ExecutionMeta::new(1_000_000, 1, account), module)
                .unwrap()
                .write_set,
        );

        let script = format!(
            "
            script {{
            use 0x{}::Store;
            fun main(val: u64) {{
                Store::store_u64(val);
            }}
            }}
        ",
            account
        );
        let script = compiler.compile(&script, Some(account)).unwrap();
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
