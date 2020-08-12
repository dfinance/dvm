use std::fmt;

use libra::{prelude::*, vm::*, gas::*};
use serde::Deserialize;

use ds::{DataSource, BlackListDataSource};
use crate::gas_schedule;

/// Stores metadata for vm execution.
#[derive(Debug)]
pub struct ExecutionMeta {
    /// Max gas units to be used in transaction execution.
    pub max_gas_amount: u64,
    /// Price in `XFI` coins per unit of gas.
    pub gas_unit_price: u64,
    /// Sender address of the transaction owner.
    pub sender: AccountAddress,
}

impl ExecutionMeta {
    /// Constructor.
    pub fn new(max_gas_amount: u64, gas_unit_price: u64, sender: AccountAddress) -> ExecutionMeta {
        ExecutionMeta {
            max_gas_amount,
            gas_unit_price,
            sender,
        }
    }

    /// Default metadata for testing.
    pub fn test() -> ExecutionMeta {
        ExecutionMeta {
            max_gas_amount: 1_000_000,
            gas_unit_price: 1,
            sender: CORE_CODE_ADDRESS,
        }
    }
}

/// Result of transaction execution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionResult {
    /// Changes to the chain.
    pub write_set: WriteSet,
    /// Emitted events.
    pub events: Vec<ContractEvent>,
    /// Number of gas units used for execution.
    pub gas_used: u64,
    /// Status of execution.
    pub status: VMError,
}

impl ExecutionResult {
    /// Creates `ExecutionResult` out of resulting chain data cache and `vm_result`.
    fn new(
        cost_strategy: CostStrategy,
        txn_meta: ExecutionMeta,
        vm_result: VMResult<TransactionEffects>,
    ) -> ExecutionResult {
        let gas_used = GasUnits::new(txn_meta.max_gas_amount)
            .sub(cost_strategy.remaining_gas())
            .get();

        vm_result
            .and_then(|effects| {
                txn_effects_to_writeset_and_events_cached(&mut (), effects).map_err(|err| {
                    PartialVMError::new(err.status_code()).finish(Location::Undefined)
                })
            })
            .map(|(write_set, events)| ExecutionResult {
                write_set,
                events,
                gas_used,
                status: PartialVMError::new(StatusCode::EXECUTED).finish(Location::Undefined),
            })
            .unwrap_or_else(|status| ExecutionResult {
                write_set: WriteSetMut::default().freeze().expect("Impossible error."),
                events: vec![],
                gas_used,
                status,
            })
    }
}

/// Result enum for ExecutionResult
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

    /// Publishes module to the chain.
    pub fn publish_module(&self, meta: ExecutionMeta, module: Module) -> VmResult {
        let mut cost_strategy =
            CostStrategy::transaction(&self.cost_table, GasUnits::new(meta.max_gas_amount));

        cost_strategy
            .charge_intrinsic_gas(AbstractMemorySize::new(module.code.len() as u64))
            .map_err(|err| err.into_vm_status())?;
        let res = CompiledModule::deserialize(module.code())
            .map_err(|e| e.finish(Location::Undefined))
            .and_then(|compiled_module| {
                let module_id = compiled_module.self_id();
                if meta.sender != *module_id.address() {
                    return Err(PartialVMError::new(
                        StatusCode::MODULE_ADDRESS_DOES_NOT_MATCH_SENDER,
                    )
                    .finish(Location::Module(module_id)));
                }

                cost_strategy
                    .charge_intrinsic_gas(AbstractMemorySize::new(module.code.len() as u64))?;

                if meta.sender == CORE_CODE_ADDRESS {
                    self.ds.clear();
                    let loader = &self.vm.runtime.loader;
                    *loader.scripts.lock().unwrap() = ScriptCache::new();
                    *loader.type_cache.lock().unwrap() = TypeCache::new();
                    *loader.module_cache.lock().unwrap() = ModuleCache::new();

                    let mut blacklist = BlackListDataSource::new(self.ds.clone());
                    blacklist.add_module(&module_id);
                    let mut session = self.vm.new_session(&blacklist);

                    session
                        .publish_module(module.code().to_vec(), meta.sender, &mut cost_strategy)
                        .and_then(|_| session.finish())
                } else {
                    let mut session = self.vm.new_session(&self.ds);
                    session
                        .publish_module(module.code().to_vec(), meta.sender, &mut cost_strategy)
                        .and_then(|_| session.finish())
                }
            });

        Ok(ExecutionResult::new(cost_strategy, meta, res))
    }

    /// Executes passed script on the chain.
    pub fn execute_script(&self, meta: ExecutionMeta, script: Script) -> VmResult {
        let mut session = self.vm.new_session(&self.ds);

        let (script, args, type_args) = script.into_inner();
        let mut cost_strategy =
            CostStrategy::transaction(&self.cost_table, GasUnits::new(meta.max_gas_amount));

        let res = session
            .execute_script(script, type_args, args, meta.sender, &mut cost_strategy)
            .and_then(|_| session.finish());

        Ok(ExecutionResult::new(cost_strategy, meta, res))
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

/// Script bytecode + passed arguments and type parameters.
pub struct Script {
    code: Vec<u8>,
    args: Vec<Value>,
    type_args: Vec<TypeTag>,
}

impl Script {
    /// Contructor.
    pub fn new(code: Vec<u8>, args: Vec<Value>, type_args: Vec<TypeTag>) -> Self {
        Script {
            code,
            args,
            type_args,
        }
    }

    /// Script bytecode.
    pub fn code(&self) -> &[u8] {
        &self.code
    }

    /// Parameters passed to main() function.
    pub fn args(&self) -> &[Value] {
        &self.args
    }

    /// Convert into internal data.
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

/// Deserializable `u64` for lcs.
#[derive(Deserialize, Debug, PartialEq, Eq)]
pub struct U64Store {
    /// Internal value.
    pub val: u64,
}

/// Deserializable `AccountAddress` for lcs.
#[derive(Deserialize, Debug, PartialEq, Eq)]
pub struct AddressStore {
    /// Internal value.
    pub val: AccountAddress,
}

/// Deserializable `Vec<u8>` for lcs.
#[derive(Deserialize, Debug, PartialEq, Eq)]
pub struct VectorU8Store {
    /// Internal value.
    pub val: Vec<u8>,
}

#[cfg(test)]
pub mod tests {
    use compiler::Compiler;
    use ds::{DataAccess, MockDataSource};
    use lang::{stdlib::zero_std};
    use libra::{prelude::*, vm::*};
    use crate::move_vm::{Dvm, ExecutionMeta, Script, U64Store};

    #[test]
    fn test_publish_module() {
        let ds = MockDataSource::with_write_set(zero_std());
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
        assert!(DataAccess::get_module(&ds, &module_id).unwrap().is_none());

        ds.merge_write_set(output.write_set);
        assert_ne!(output.gas_used, 0);

        let loaded_module = DataAccess::get_module(&ds, &module_id).unwrap().unwrap();
        assert_eq!(loaded_module, module);

        //try public module duplicate;
        assert_eq!(
            StatusCode::DUPLICATE_MODULE_NAME,
            vm.publish_module(ExecutionMeta::new(1_000_000, 1, account), module)
                .unwrap()
                .status
                .major_status()
        );
    }

    #[test]
    fn test_execute_script() {
        let ds = MockDataSource::with_write_set(zero_std());
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
            fun main(account: &signer, val: u64) {{
                Store::store_u64(account, val);
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
