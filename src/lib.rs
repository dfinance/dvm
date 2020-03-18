#[macro_use]
extern crate vm_runtime;
#[macro_use]
extern crate anyhow;

pub mod compiler;
pub mod ds;
pub mod service;
#[macro_use]
pub mod vm;

pub mod test_kit;

// reshare libra crates
pub use libra_types;

mod api_grpc_ext;
// TODO: [REF] rename to api_grpc
pub mod compiled_protos {
    extern crate dvm_api;
    pub use dvm_api::grpc::*;
    pub use crate::api_grpc_ext::*;
}
