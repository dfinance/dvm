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
