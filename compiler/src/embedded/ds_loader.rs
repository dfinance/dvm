use crate::mv::dependence::loader::BytecodeLoader;
use anyhow::Result;
use libra::prelude::*;

#[derive(Clone)]
pub struct RemoteCacheLoader<C: RemoteCache + Clone> {
    cache: C,
}

impl<C> RemoteCacheLoader<C>
where
    C: RemoteCache + Clone,
{
    pub fn new(view: C) -> RemoteCacheLoader<C> {
        RemoteCacheLoader { cache: view }
    }
}

impl<C> BytecodeLoader for RemoteCacheLoader<C>
where
    C: RemoteCache + Clone,
{
    fn load(&self, module_id: &ModuleId) -> Result<Vec<u8>> {
        if let Some(bytecode) = self
            .cache
            .get_module(&module_id)
            .map_err(|err| err.into_vm_status())?
        {
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
