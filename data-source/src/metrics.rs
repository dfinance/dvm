use dvm_info::metrics::execution::ExecutionResult;
use dvm_info::metrics::meter::ScopeMeter;
use libra::prelude::*;

use crate::{Clear, DataSource};

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

impl<D> Clear for DsMeter<D>
where
    D: DataSource,
{
    fn clear(&self) {
        self.inner.clear();
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
        tag: &TypeTag,
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
