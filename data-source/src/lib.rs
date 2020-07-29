//! Interface between MoveVM `StateView` implementation and gRPC API for `dnode`.

#![warn(missing_docs)]

#[macro_use]
extern crate anyhow;
#[macro_use]
extern crate log;

/// `GrpcDataSource` to wrap all gRPC calls to `dnode`.
pub mod grpc;

/// Defines `DsMeter` which implements `StateView` and adds metric recording for all `StateView` method calls.
pub mod metrics;

/// `MockDataSource` to be used in test_kit.
pub mod mock;

/// Defines `ModuleCache` which implements caching for fetching modules from `dnode`.
pub mod module_cache;

/// Defines `BlackListDataSource` which provides implements blacklist of access path.
pub mod blacklist;

use libra::prelude::*;

use anyhow::Error;

pub use mock::MockDataSource;
pub use module_cache::ModuleCache;
pub use metrics::DsMeter;
pub use grpc::GrpcDataSource;
pub use blacklist::{BlackListDataSource};

/// Thread-safe `StateView`.
pub trait DataSource: StateView + RemoteCache + Clear + Clone + Send + Sync + 'static {}

/// Used to automatically implement `get_module` which calls `StateView.get()`
/// internally and automatically wraps result with `Module`.
pub trait DataAccess {
    /// See autoimplementation of the trait for all `StateView` objects.
    fn get_module(&self, module_id: &ModuleId) -> Result<Option<Module>, Error>;
}

/// Trait to `clear()` internal data structure.
pub trait Clear {
    /// No-op in default implementation.
    /// Called on internal `DataSource` object to remove all entries from internal cache.
    /// Used when `sender` is the built-in 0x0 / 0x1 address.
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
