#![feature(never_type)]

#[macro_use]
extern crate lazy_static;

use crate::grpc::server;
use language_e2e_tests::data_store::FakeDataStore;
use vm_runtime::chain_state::TransactionExecutionContext;

mod grpc;
mod service;
mod vm;

fn main() -> Result<!, String> {
    let vm = vm::MoveVm::new(Box::new(FakeDataStore::default()));
    let _serv = server::run_async(service::VM::new())?;

    // ...

    std::thread::park();
    unreachable!();
}