use anyhow::Error;
use libra::libra_state_view::StateView;
use libra::libra_types::access_path::AccessPath;
use libra::libra_types::language_storage::ModuleId;
use crate::bytecode::disassembler::ModuleSignature;
use crate::bytecode::disassembler;

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

    fn load_module_signature(&self, module_id: &ModuleId) -> Result<ModuleSignature, Error> {
        let path = AccessPath::code_access_path(&module_id);
        if let Some(blob) = self.state_view.get(&path)? {
            Ok(disassembler::module_signature(&blob)?)
        } else {
            Err(Error::msg(format!(
                "Module with path [{:?}] not found",
                module_id
            )))
        }
    }

    pub fn load_modules_signature(&self, ids: &[ModuleId]) -> Result<Vec<ModuleSignature>, Error> {
        ids.iter()
            .map(|dep| self.load_module_signature(dep))
            .collect()
    }
}
