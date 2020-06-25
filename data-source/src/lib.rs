#[macro_use]
extern crate anyhow;
#[macro_use]
extern crate log;

/// `GrpcDataSource` to wrap all gRPC calls to `dnode`.
pub mod grpc;
pub mod metrics;
pub mod mock;
pub mod module_cache;

use libra::{libra_types, libra_state_view, move_vm_runtime};
use libra_types::write_set::WriteSet;
use libra::move_core_types::language_storage::ModuleId;
use libra_types::transaction::Module;
use libra_types::access_path::AccessPath;
use libra_state_view::StateView;
use anyhow::Error;

pub use mock::MockDataSource;
pub use module_cache::ModuleCache;
pub use metrics::DsMeter;
pub use grpc::GrpcDataSource;
use move_vm_runtime::data_cache::RemoteCache;

pub trait DataSource: StateView + RemoteCache + Clear + Clone + Send + Sync + 'static {}

pub trait MergeWriteSet {
    fn merge_write_set(&self, write_set: WriteSet);
}

pub trait DataAccess {
    fn get_module(&self, module_id: &ModuleId) -> Result<Option<Module>, Error>;
}

pub trait Clear {
    fn clear(&self) {
        //no-op
    }
}

// auto-impl for all StateView impls
impl<T: StateView> DataAccess for T {
    fn get_module(&self, module_id: &ModuleId) -> Result<Option<Module>, Error> {
        let entry = self.get(&AccessPath::from(module_id))?;
        Ok(entry.map(Module::new))
    }
}
