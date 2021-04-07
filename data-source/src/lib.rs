//! Interface between MoveVM `StateView` implementation and gRPC API for `dnode`.

#![warn(missing_docs)]

#[macro_use]
extern crate anyhow;
#[macro_use]
extern crate log;

use anyhow::Error;
use serde::{Deserialize, Serialize};

pub use blacklist::BlackListDataSource;
pub use grpc::GrpcDataSource;
use libra::prelude::*;
pub use metrics::DsMeter;
pub use mock::MockDataSource;
pub use module_cache::ModuleCache;

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

/// Thread-safe `StateView`.
pub trait DataSource:
    RemoteCache + Oracle + Balance + RemoveModule + GetCurrencyInfo + Clone + Send + Sync + 'static
{
}

/// Oracle access.
pub trait Oracle {
    /// Results price of `currency_2` in `currency_1`.
    fn get_price(&self, currency_1: String, currency_2: String) -> Result<Option<u128>, Error>;
}

/// Balance access.
pub trait Balance {
    /// Returns balance of account with `address` and `ticker`.
    fn get_balance(&self, address: AccountAddress, ticker: String) -> Result<Option<u128>, Error>;
}

/// CurrencyInfo request.
pub trait GetCurrencyInfo {
    /// Returns info abort currency with `ticker`.
    fn get_currency_info(&self, ticker: String) -> Result<Option<CurrencyInfo>, Error>;
}

/// Currency info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyInfo {
    /// Denom.
    pub denom: Vec<u8>,
    /// Decimals.
    pub decimals: u8,
    /// Is token.
    pub is_token: bool,
    /// Owner address.
    pub address: AccountAddress,
    /// Total supply.
    pub total_supply: u128,
}

/// Trait to `remove_module` internal data structure.
pub trait RemoveModule {
    /// Removes the module by its id.
    fn remove_module(&self, _module_id: &ModuleId) {
        //no-op
    }
}
