use crate::{DataSource, Clear};
use std::collections::HashSet;
use libra::prelude::*;
use anyhow::Error;

/// Wrapper for data source which returns blank for requests from blacklist.
#[derive(Debug, Clone)]
pub struct BlackListDataSource<D>
where
    D: DataSource,
{
    inner: D,
    blacklist: HashSet<AccessPath>,
}

impl<D> BlackListDataSource<D>
where
    D: DataSource,
{
    /// Create a new BlackListDataSource with DataSource.
    pub fn new(inner: D) -> BlackListDataSource<D> {
        BlackListDataSource {
            inner,
            blacklist: Default::default(),
        }
    }

    /// Add module to the blacklist.
    pub fn add_module(&mut self, module_id: &ModuleId) {
        self.blacklist.insert(AccessPath::from(module_id));
    }

    /// Add resource to the blacklist.
    pub fn add_resource(&mut self, address: &AccountAddress, tag: &TypeTag) {
        if let TypeTag::Struct(struct_tag) = tag.clone() {
            let resource_tag = ResourceKey::new(*address, struct_tag);
            self.blacklist
                .insert(AccessPath::resource_access_path(&resource_tag));
        }
    }
}

impl<D> StateView for BlackListDataSource<D>
where
    D: DataSource,
{
    fn get(&self, access_path: &AccessPath) -> Result<Option<Vec<u8>>, Error> {
        if self.blacklist.contains(access_path) {
            Ok(None)
        } else {
            self.inner.get(access_path)
        }
    }

    fn multi_get(&self, access_paths: &[AccessPath]) -> Result<Vec<Option<Vec<u8>>>, Error> {
        access_paths
            .iter()
            .map(|path| StateView::get(self, path))
            .collect()
    }

    fn is_genesis(&self) -> bool {
        self.inner.is_genesis()
    }
}

impl<D> RemoteCache for BlackListDataSource<D>
where
    D: DataSource,
{
    fn get_module(&self, module_id: &ModuleId) -> VMResult<Option<Vec<u8>>> {
        RemoteStorage::new(self).get_module(module_id)
    }

    fn get_resource(
        &self,
        address: &AccountAddress,
        tag: &TypeTag,
    ) -> PartialVMResult<Option<Vec<u8>>> {
        RemoteStorage::new(self).get_resource(address, tag)
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
