use crate::{DataSource, Clear};
use std::collections::HashSet;
use libra::prelude::*;

/// Wrapper for data source which returns blank for requests from blacklist.
#[derive(Debug, Clone)]
pub struct BlackListDataSource<D>
where
    D: DataSource,
{
    inner: D,
    modules: HashSet<ModuleId>,
    resources: HashSet<(AccountAddress, TypeTag)>,
}

impl<D> BlackListDataSource<D>
where
    D: DataSource,
{
    /// Create a new BlackListDataSource with DataSource.
    pub fn new(inner: D) -> BlackListDataSource<D> {
        BlackListDataSource {
            inner,
            modules: Default::default(),
            resources: Default::default(),
        }
    }

    /// Add module to the blacklist.
    pub fn add_module(&mut self, module_id: &ModuleId) {
        self.modules.insert(module_id.to_owned());
    }

    /// Add resource to the blacklist.
    pub fn add_resource(&mut self, address: &AccountAddress, tag: &TypeTag) {
        self.resources.insert((*address, tag.to_owned()));
    }
}

impl<D> RemoteCache for BlackListDataSource<D>
where
    D: DataSource,
{
    fn get_module(&self, module_id: &ModuleId) -> VMResult<Option<Vec<u8>>> {
        if self.modules.contains(module_id) {
            return Ok(None);
        } else {
            self.inner.get_module(module_id)
        }
    }

    fn get_resource(
        &self,
        address: &AccountAddress,
        tag: &TypeTag,
    ) -> PartialVMResult<Option<Vec<u8>>> {
        if self.resources.contains(&(*address, tag.to_owned())) {
            return Ok(None);
        } else {
            self.inner.get_resource(address, tag)
        }
    }
}

impl<D> DataSource for BlackListDataSource<D> where D: DataSource {}

impl<D> Clear for BlackListDataSource<D>
where
    D: DataSource,
{
    fn clear(&self) {
        self.inner.clear();
    }
}
