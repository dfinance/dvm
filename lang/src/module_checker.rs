use libra::move_vm_runtime::MoveVM;
use libra::libra_vm::CompiledModule;
use libra::bytecode_verifier::VerifiedModule;
use anyhow::{anyhow, Error};
use libra::libra_types::vm_error::{VMStatus, StatusCode};
use libra::move_vm_state::execution_context::TransactionExecutionContext;
use ds::MockDataSource;
use libra::move_core_types::gas_schedule::{GasUnits, GasAlgebra};
use libra::libra_types::language_storage::ModuleId;
use libra::libra_types::account_address::AccountAddress;
use libra::move_core_types::identifier::Identifier;

#[derive(Default)]
pub struct ModuleChecker {
    vm: MoveVM,
    ds: MockDataSource,
}

impl ModuleChecker {
    pub fn new() -> Self {
        ModuleChecker {
            vm: MoveVM::new(),
            ds: MockDataSource::new(),
        }
    }

    pub fn check(&self, bytecode: &[u8]) -> Result<(), (ModuleId, VMStatus)> {
        let module = CompiledModule::deserialize(&bytecode).map_err(|err| {
            (
                ModuleId::new(
                    AccountAddress::default(),
                    Identifier::new("Unknown").unwrap(),
                ),
                err,
            )
        })?;

        let id = module.self_id();
        VerifiedModule::new(module)
            .map_err(|err| err.1)
            .and_then(|module| {
                let mut context =
                    TransactionExecutionContext::new(GasUnits::new(1_000_000), &self.ds);
                self.vm.cache_module(module, &mut context)
            })
            .and_then(|_| {
                self.ds.publish_module(bytecode.to_owned()).map_err(|err| {
                    VMStatus::new(StatusCode::REMOTE_DATA_ERROR).with_message(err.to_string())
                })?;
                Ok(())
            })
            .map_err(|err| (id, err))
    }

    pub fn check_with_verbal_error(&self, bytecode: &[u8]) -> Result<(), Error> {
        self.check(bytecode).map_err(|(id, status)| {
            anyhow!("Invalid module:{:?}; Check status: [error:{:?}, code:{}, sub_status:{}, message: {}]",
            id,
            status.major_status,
            status.major_status as u32,
            status.sub_status.unwrap_or(0),
            status.message.unwrap_or_else(|| "".to_owned()))
        })
    }
}

#[cfg(test)]
pub mod tests {
    use crate::module_checker::ModuleChecker;
    use ds::MockDataSource;
    use libra::libra_types::{account_address::AccountAddress, vm_error::StatusCode};
    use libra::libra_types::language_storage::ModuleId;
    use libra::move_core_types::identifier::Identifier;
    use crate::compiler::Compiler;
    use libra::libra_types::vm_error::VMStatus;

    #[test]
    fn test_missing_native_function() {
        let checker = ModuleChecker::new();
        let ds = MockDataSource::new();
        let compiler = Compiler::new(ds);

        let module = r"
            module FakeNative {
                native public fun empty();
            }
        ";
        let bytecode = compiler
            .compile(module, &AccountAddress::default())
            .unwrap();
        let error = checker.check(&bytecode).unwrap_err();
        assert_eq!(
            error,
            (
                ModuleId::new(
                    AccountAddress::default(),
                    Identifier::new("FakeNative").unwrap(),
                ),
                VMStatus::new(StatusCode::MISSING_DEPENDENCY)
                    .with_message("at index 0 while indexing function handle".to_owned())
            )
        );
    }

    #[test]
    fn green_test() {
        let checker = ModuleChecker::new();
        let ds = MockDataSource::new();
        let compiler = Compiler::new(ds);

        let module = r"
           module Hash {
                native public fun sha2_256(data: vector<u8>): vector<u8>;
                native public fun sha3_256(data: vector<u8>): vector<u8>;
           }
        ";
        let bytecode = compiler
            .compile(module, &AccountAddress::default())
            .unwrap();
        checker.check(&bytecode).unwrap();
    }
}
