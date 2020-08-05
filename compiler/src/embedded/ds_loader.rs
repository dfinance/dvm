use crate::mv::dependence::loader::BytecodeLoader;
use anyhow::Result;
use libra::prelude::*;

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

impl<S> BytecodeLoader for StateViewLoader<S>
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
