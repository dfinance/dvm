use libra::bytecode_verifier::VerifiedModule;
use anyhow::Error;
use libra::libra_state_view::StateView;
use libra::libra_types::access_path::AccessPath;
use libra::vm::CompiledModule;
use libra::libra_types::language_storage::ModuleId;

#[derive(Clone)]
pub struct ModuleLoader<S>
where
    S: StateView + Clone,
{
    state_view: S,
}

impl<S> ModuleLoader<S>
where
    S: StateView + Clone,
{
    pub fn new(state_view: S) -> ModuleLoader<S> {
        ModuleLoader { state_view }
    }

    fn load_module(&self, module_id: &ModuleId) -> Result<VerifiedModule, Error> {
        let path = AccessPath::code_access_path(&module_id);
        if let Some(blob) = self.state_view.get(&path)? {
            let module = CompiledModule::deserialize(&blob).map_err(|err| {
                Error::msg(format!(
                    "Module [{:?}] deserialization failed:[{:?}]",
                    module_id, err
                ))
            })?;
            let module = VerifiedModule::new(module).map_err(|err| {
                Error::msg(format!(
                    "Failed to verify module [{:?}]; [{:?}]",
                    module_id, err
                ))
            })?;
            Ok(module)
        } else {
            Err(Error::msg(format!(
                "Module with path [{:?}] not found",
                module_id
            )))
        }
    }

    pub fn load_modules(&self, ids: &[ModuleId]) -> Result<Vec<VerifiedModule>, Error> {
        ids.iter().map(|dep| self.load_module(dep)).collect()
    }
}
