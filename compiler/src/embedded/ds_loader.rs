use crate::mv::dependence::loader::BytecodeSource;
use anyhow::Result;
use libra::libra_state_view::StateView;
use libra::libra_types::access_path::AccessPath;
use libra::libra_types::language_storage::ModuleId;

#[derive(Clone)]
pub struct StateViewLoader<S: StateView + Clone> {
    view: S,
}

impl<S> StateViewLoader<S>
where
    S: StateView + Clone,
{
    pub fn new(view: S) -> StateViewLoader<S> {
        StateViewLoader { view }
    }
}

impl<S> BytecodeSource for StateViewLoader<S>
where
    S: StateView + Clone,
{
    fn load(&self, module_id: &ModuleId) -> Result<Vec<u8>> {
        let path = AccessPath::code_access_path(module_id);
        if let Some(bytecode) = self.view.get(&path)? {
            Ok(bytecode)
        } else {
            Err(anyhow!(
                "Module '0x{}::{}' not found",
                module_id.address(),
                module_id.name()
            ))
        }
    }
}
