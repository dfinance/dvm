use crate::vm::types::*;
use libra::{prelude::*, vm::*, gas::*};
use std::fmt;
use crate::gas_schedule;
use ds::{DataSource, BlackListDataSource};

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
    pub fn publish_module(&self, gas: Gas, module: ModuleTx) -> VmResult {
        let (module, sender) = module.into_inner();

        let mut cost_strategy =
            CostStrategy::transaction(&self.cost_table, GasUnits::new(gas.max_gas_amount()));

        cost_strategy
            .charge_intrinsic_gas(AbstractMemorySize::new(module.len() as u64))
            .map_err(|err| err.into_vm_status())?;
        let res = CompiledModule::deserialize(&module)
            .map_err(|e| e.finish(Location::Undefined))
            .and_then(|compiled_module| {
                let module_id = compiled_module.self_id();
                if sender != *module_id.address() {
                    return Err(PartialVMError::new(
                        StatusCode::MODULE_ADDRESS_DOES_NOT_MATCH_SENDER,
                    )
                    .finish(Location::Module(module_id)));
                }

                cost_strategy.charge_intrinsic_gas(AbstractMemorySize::new(module.len() as u64))?;

                if sender == CORE_CODE_ADDRESS {
                    self.ds.clear();
                    let loader = &self.vm.runtime.loader;
                    *loader.scripts.lock().unwrap() = ScriptCache::new();
                    *loader.type_cache.lock().unwrap() = TypeCache::new();
                    *loader.module_cache.lock().unwrap() = ModuleCache::new();

                    let mut blacklist = BlackListDataSource::new(self.ds.clone());
                    blacklist.add_module(&module_id);
                    let mut session = self.vm.new_session(&blacklist);

                    session
                        .publish_module(module.to_vec(), sender, &mut cost_strategy)
                        .and_then(|_| session.finish())
                } else {
                    let mut session = self.vm.new_session(&self.ds);
                    session
                        .publish_module(module.to_vec(), sender, &mut cost_strategy)
                        .and_then(|_| session.finish())
                }
            });

        Ok(ExecutionResult::new(cost_strategy, gas, res))
    }

    /// Executes passed script on the chain.
    pub fn execute_script(&self, gas: Gas, tx: ScriptTx) -> VmResult {
        let mut session = self.vm.new_session(&self.ds);

        let (script, args, type_args, senders) = tx.into_inner();
        let mut cost_strategy =
            CostStrategy::transaction(&self.cost_table, GasUnits::new(gas.max_gas_amount()));

        let res = session
            .execute_script(script, type_args, args, senders, &mut cost_strategy)
            .and_then(|_| session.finish());

        Ok(ExecutionResult::new(cost_strategy, gas, res))
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
