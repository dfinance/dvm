use crate::{DataSource, RemoveModule, Oracle, Balance, GetCurrencyInfo, CurrencyInfo};
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
    modules: HashSet<ModuleId>,
    resources: HashSet<(AccountAddress, StructTag)>,
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
    pub fn add_resource(&mut self, address: &AccountAddress, tag: &StructTag) {
        self.resources.insert((*address, tag.to_owned()));
    }
}

impl<D> RemoteCache for BlackListDataSource<D>
where
    D: DataSource,
{
    fn get_module(&self, module_id: &ModuleId) -> VMResult<Option<Vec<u8>>> {
        if self.modules.contains(module_id) {
            Ok(None)
        } else {
            self.inner.get_module(module_id)
        }
    }

    fn get_resource(
        &self,
        address: &AccountAddress,
        tag: &StructTag,
    ) -> PartialVMResult<Option<Vec<u8>>> {
        if self.resources.contains(&(*address, tag.to_owned())) {
            Ok(None)
        } else {
            self.inner.get_resource(address, tag)
        }
    }
}

impl<D: DataSource> Oracle for BlackListDataSource<D> {
    fn get_price(&self, currency_1: String, currency_2: String) -> Result<Option<u128>, Error> {
        self.inner.get_price(currency_1, currency_2)
    }
}

impl<D: DataSource> Balance for BlackListDataSource<D> {
    fn get_balance(&self, address: AccountAddress, ticker: String) -> Result<Option<u128>, Error> {
        self.inner.get_balance(address, ticker)
    }
}

impl<D: DataSource> GetCurrencyInfo for BlackListDataSource<D> {
    fn get_currency_info(&self, ticker: String) -> Result<Option<CurrencyInfo>, Error> {
        self.inner.get_currency_info(ticker)
    }
}

impl<D> DataSource for BlackListDataSource<D> where D: DataSource {}

impl<D> RemoveModule for BlackListDataSource<D>
where
    D: DataSource,
{
    fn remove_module(&self, module_id: &ModuleId) {
        self.inner.remove_module(module_id)
    }
}
