#[macro_use]
extern crate anyhow;
extern crate libra;

pub mod compiler;
pub mod ds;
pub mod service;
pub mod vm;

mod api_grpc_ext;
// TODO: [REF] rename to api_grpc
pub mod compiled_protos {
    extern crate dvm_api;
    pub use dvm_api::grpc::*;
    pub use crate::api_grpc_ext::*;
}

use anyhow::Result;

pub fn get_sentry_dsn() -> Result<String> {
    std::env::var("SENTRY_DSN")
        .map_err(|_| anyhow!("SENTRY_DSN environment variable is not provided, Sentry integration is going to be disabled"))
}
