use anyhow::Error;

use dvm_info::metrics::execution::ExecutionResult;
use dvm_info::metrics::meter::ScopeMeter;
use libra::prelude::*;

use crate::{Balance, CurrencyInfo, DataSource, GetCurrencyInfo, Oracle, RemoveModule};

/// Wrapper for data source which collects metrics queries.
#[derive(Debug, Clone)]
pub struct DsMeter<D>
where
    D: DataSource,
{
    inner: D,
}

impl<D> DsMeter<D>
where
    D: DataSource,
{
    /// Constructor
    pub fn new(inner: D) -> DsMeter<D> {
        DsMeter { inner }
    }
}

impl<D> RemoveModule for DsMeter<D>
where
    D: DataSource,
{
    fn remove_module(&self, module_id: &ModuleId) {
        self.inner.remove_module(module_id)
    }
}

impl<D> RemoteCache for DsMeter<D>
where
    D: DataSource,
{
    fn get_module(&self, module_id: &ModuleId) -> VMResult<Option<Vec<u8>>> {
        let mut meter = ScopeMeter::new("ds_access");
        match self.inner.get_module(module_id) {
            Ok(Some(data)) => {
                meter.set_result(ExecutionResult::new(true, 200, data.len() as u64));
                Ok(Some(data))
            }
            Ok(None) => {
                meter.set_result(ExecutionResult::new(false, 404, 0));
                Ok(None)
            }
            Err(err) => {
                meter.set_result(ExecutionResult::new(false, 500, 0));
                Err(err)
            }
        }
    }

    fn get_resource(
        &self,
        address: &AccountAddress,
        tag: &StructTag,
    ) -> PartialVMResult<Option<Vec<u8>>> {
        let mut meter = ScopeMeter::new("ds_access");
        match self.inner.get_resource(address, tag) {
            Ok(Some(data)) => {
                meter.set_result(ExecutionResult::new(true, 200, data.len() as u64));
                Ok(Some(data))
            }
            Ok(None) => {
                meter.set_result(ExecutionResult::new(false, 404, 0));
                Ok(None)
            }
            Err(err) => {
                meter.set_result(ExecutionResult::new(false, 500, 0));
                Err(err)
            }
        }
    }
}

impl<D> DataSource for DsMeter<D> where D: DataSource {}

impl<D> Balance for DsMeter<D>
where
    D: DataSource,
{
    fn get_balance(&self, address: AccountAddress, ticker: String) -> Result<Option<u128>, Error> {
        let mut meter = ScopeMeter::new("balance_access");
        match self.inner.get_balance(address, ticker) {
            Ok(Some(data)) => {
                meter.set_result(ExecutionResult::new(true, 200, 0));
                Ok(Some(data))
            }
            Ok(None) => {
                meter.set_result(ExecutionResult::new(false, 404, 0));
                Ok(None)
            }
            Err(err) => {
                meter.set_result(ExecutionResult::new(false, 500, 0));
                Err(err)
            }
        }
    }
}

impl<D> Oracle for DsMeter<D>
where
    D: DataSource,
{
    fn get_price(&self, currency_1: String, currency_2: String) -> Result<Option<u128>, Error> {
        let mut meter = ScopeMeter::new("oracle_access");
        match self.inner.get_price(currency_1, currency_2) {
            Ok(Some(data)) => {
                meter.set_result(ExecutionResult::new(true, 200, 0));
                Ok(Some(data))
            }
            Ok(None) => {
                meter.set_result(ExecutionResult::new(false, 404, 0));
                Ok(None)
            }
            Err(err) => {
                meter.set_result(ExecutionResult::new(false, 500, 0));
                Err(err)
            }
        }
    }
}

impl<D> GetCurrencyInfo for DsMeter<D>
where
    D: DataSource,
{
    fn get_currency_info(&self, ticker: String) -> Result<Option<CurrencyInfo>, Error> {
        let mut meter = ScopeMeter::new("currency_info_access");
        match self.inner.get_currency_info(ticker) {
            Ok(Some(data)) => {
                meter.set_result(ExecutionResult::new(true, 200, 0));
                Ok(Some(data))
            }
            Ok(None) => {
                meter.set_result(ExecutionResult::new(false, 404, 0));
                Ok(None)
            }
            Err(err) => {
                meter.set_result(ExecutionResult::new(false, 500, 0));
                Err(err)
            }
        }
    }
}
