use std::fmt;
use std::sync::RwLock;

use ds::{BlackListDataSource, DataSource};
use dvm_info::memory_check::MemoryChecker;
use libra::{gas::*, prelude::*, vm::*};

use crate::gas_schedule;
use crate::vm::session::StateViewSession;
use crate::vm::types::*;

/// Dfinance virtual machine.
pub struct Dvm<D: DataSource> {
    /// Libra virtual machine.
    vm: RwLock<MoveVM>,
    /// Data source.
    ds: D,
    /// Instructions cost table.
    cost_table: CostTable,
    /// Dvm memory checker.
    mem_checker: Option<MemoryChecker>,
}

impl<D> Dvm<D>
where
    D: DataSource,
{
    /// Create a new virtual machine with the given data source.
    pub fn new(ds: D, mem_checker: Option<MemoryChecker>) -> Dvm<D> {
        let vm = RwLock::new(MoveVM::new());
        trace!("vm service is ready.");
        Dvm {
            vm,
            ds,
            cost_table: gas_schedule::cost_table(),
            mem_checker,
        }
    }

    /// Publishes module to the chain.
    pub fn publish_module(&self, gas: Gas, module: ModuleTx) -> VmResult {
        self.perform_memory_prevention();

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
                    self.ds.remove_module(&module_id);
                    self.clear_cache();

                    let mut blacklist = BlackListDataSource::new(self.ds.clone());
                    blacklist.add_module(&module_id);
                    let vm = self.vm.read().unwrap();
                    let (sv, bank) = StateViewSession::session(&blacklist, 0, 0);
                    let mut session = vm.new_session(&sv, bank);

                    session
                        .publish_module(
                            module.to_vec(),
                            sender,
                            &mut cost_strategy,
                            &NoContextLog::new(),
                        )
                        .and_then(|_| session.finish())
                } else {
                    let vm = self.vm.read().unwrap();
                    let (sv, bank) = StateViewSession::session(&self.ds, 0, 0);
                    let mut session = vm.new_session(&sv, bank);
                    session
                        .publish_module(
                            module.to_vec(),
                            sender,
                            &mut cost_strategy,
                            &NoContextLog::new(),
                        )
                        .and_then(|_| session.finish())
                }
            });

        Ok(ExecutionResult::new(cost_strategy, gas, res))
    }

    fn clear_cache(&self) {
        let new_vm = MoveVM::new();
        let mut vm = self.vm.write().unwrap_or_else(|err| err.into_inner());
        *vm = new_vm;
    }

    fn perform_memory_prevention(&self) {
        if let Some(mem_checker) = &self.mem_checker {
            if mem_checker.is_limit_exceeded() {
                self.clear_cache();
            }
        }
    }

    /// Executes passed script on the chain.
    pub fn execute_script(&self, gas: Gas, tx: ScriptTx) -> VmResult {
        self.perform_memory_prevention();
        let vm = self.vm.read().unwrap();
        let (script, args, type_args, senders, timestamp, block) = tx.into_inner();

        let (sv, bank) = StateViewSession::session(&self.ds, timestamp, block);

        let mut session = vm.new_session(&sv, bank);

        let mut cost_strategy =
            CostStrategy::transaction(&self.cost_table, GasUnits::new(gas.max_gas_amount()));

        let res = session
            .execute_script(
                script,
                type_args,
                args,
                senders,
                &mut cost_strategy,
                &NoContextLog::new(),
            )
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
