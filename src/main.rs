#![feature(never_type)]

#[macro_use]
extern crate lazy_static;

use crate::grpc::server;
use language_e2e_tests::data_store::FakeDataStore;

mod grpc;
mod service;
mod vm;

fn main() -> Result<!, String> {
    let _vm = vm::MoveVm::new(Box::new(FakeDataStore::default()));
    server::run(service::VM::new())?;
}
