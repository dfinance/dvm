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

pub use mock::MockDataSource;
pub use module_cache::ModuleCache;
pub use metrics::DsMeter;
pub use grpc::GrpcDataSource;
pub use blacklist::{BlackListDataSource};

/// Thread-safe `StateView`.
pub trait DataSource: RemoteCache + RemoveModule + Clone + Send + Sync + 'static {}

/// Trait to `remove_module` internal data structure.
pub trait RemoveModule {
    /// Removes the module by its id.
    fn remove_module(&self, _module_id: &ModuleId) {
        //no-op
    }
}
