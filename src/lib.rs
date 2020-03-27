// TODO: #![warn(missing_docs)]

#[macro_use]
pub extern crate log;
extern crate anyhow;
extern crate libra;

pub mod cli;
pub mod services;
pub mod vm;

// TODO: [REF] rename to api_grpc
pub mod compiled_protos {
    extern crate dvm_api;
    pub use dvm_api::grpc::*;
}
