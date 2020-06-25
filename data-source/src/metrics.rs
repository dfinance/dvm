use anyhow::Error;

use dvm_info::metrics::execution::ExecutionResult;
use dvm_info::metrics::meter::ScopeMeter;
use libra::libra_state_view::StateView;
use libra::libra_types::access_path::AccessPath;
use libra::libra_vm::errors::VMResult;
use libra::move_vm_runtime::data_cache::RemoteCache;

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
    /// Contructor
    pub fn new(inner: D) -> DsMeter<D> {
        DsMeter { inner }
    }
}

impl<D> StateView for DsMeter<D>
where
    D: DataSource,
{
    fn get(&self, access_path: &AccessPath) -> Result<Option<Vec<u8>>, Error> {
        let mut meter = ScopeMeter::new("ds_access");
        match StateView::get(&self.inner, access_path) {
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
    fn get(&self, access_path: &AccessPath) -> VMResult<Option<Vec<u8>>> {
        RemoteCache::get(&self.inner, access_path)
    }
}

impl<D> DataSource for DsMeter<D> where D: DataSource {}
