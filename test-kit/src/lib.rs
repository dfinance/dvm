#![deny(missing_docs)]

//! # test-kit
//! The `test-kit` crate provides functionality for testing a virtual machine, compiler, and gRPC services.

#[macro_use]
extern crate anyhow;

mod genesis;
mod grpc_client;
mod grpc_server;
/// Move test framework.
pub mod test_suite;

pub use grpc_server::{Server, Signal};
use std::sync::{Mutex, Arc};
use std::ops::Range;
use runtime::vm::types::Gas;
use libra::prelude::*;
use std::convert::TryFrom;
use crate::grpc_client::Client;
use data_source::MockDataSource;
use lang::{
    stdlib::{build_std, zero_std},
};
use compiler::Compiler;
pub use genesis::genesis_write_set;
use crate::compiled_protos::vm_grpc::{VmArgs, VmPublishModule, VmExecuteResponse};
use dvm_net::api::grpc::vm_grpc::{VmExecuteScript, StructIdent};

/// gRPC protocol API.
pub mod compiled_protos {
    extern crate dvm_net;

    pub use dvm_net::api::grpc::*;
}

/// gRPC server ports pool.
pub const PORT_RANGE: Range<u32> = 3000..5000;

/// Arc<Mutex<>> type alias.
pub type ArcMut<T> = Arc<Mutex<T>>;

/// DVM test kit;
pub struct TestKit {
    data_source: MockDataSource,
    client: Client,
    _server: Server,
    compiler: Compiler<MockDataSource>,
}

impl Default for TestKit {
    fn default() -> Self {
        Self::new()
    }
}

impl TestKit {
    /// Creates a new test kit with stdlib.
    pub fn new() -> TestKit {
        Self::with_genesis(build_std())
    }

    /// Creates a new test kit without stdlib.
    pub fn empty() -> Self {
        Self::with_genesis(zero_std())
    }

    /// Creates a new test kit with given write set.
    pub fn with_genesis(ws: WriteSet) -> TestKit {
        let data_source = MockDataSource::with_write_set(ws);
        let server = Server::new(data_source.clone());
        let client = Client::new(server.port()).unwrap_or_else(|_| {
            panic!(
                "Client couldn't connect to the server at http://localhost:{}",
                server.port()
            )
        });

        TestKit {
            data_source: data_source.clone(),
            _server: server,
            compiler: Compiler::new(data_source),
            client,
        }
    }

    /// Publish module.
    pub fn publish_module(
        &self,
        code: &str,
        gas: Gas,
        sender: AccountAddress,
    ) -> VmExecuteResponse {
        let module = self.compiler.compile(code, Some(sender)).unwrap();
        self.client.publish_module(VmPublishModule {
            sender: sender.to_vec(),
            max_gas_amount: gas.max_gas_amount(),
            gas_unit_price: gas.gas_unit_price(),
            code: module,
        })
    }

    /// Publish module.
    pub fn publish_module_raw(
        &self,
        bytecode: Vec<u8>,
        max_gas_amount: u64,
        gas_unit_price: u64,
        sender: Vec<u8>,
    ) -> VmExecuteResponse {
        self.client.publish_module(VmPublishModule {
            sender,
            max_gas_amount,
            gas_unit_price,
            code: bytecode,
        })
    }

    /// Add std module to data source.
    pub fn add_std_module(&self, code: &str) {
        let module = self
            .compiler
            .compile(code, Some(CORE_CODE_ADDRESS))
            .unwrap();

        let id = CompiledModule::deserialize(&module).unwrap().self_id();
        self.data_source.insert((&id).into(), module);
    }

    /// Compiler source codes.
    pub fn compile(&self, code: &str, address: Option<AccountAddress>) -> anyhow::Result<Vec<u8>> {
        self.compiler.compile(code, address)
    }

    /// Execute script.
    pub fn execute_script(
        &self,
        code: &str,
        gas: Gas,
        args: Vec<VmArgs>,
        type_params: Vec<StructIdent>,
        senders: Vec<AccountAddress>,
    ) -> VmExecuteResponse {
        assert!(!senders.is_empty());
        let code = self.compiler.compile(code, Some(senders[0])).unwrap();

        let senders = senders.iter().map(|sender| sender.to_vec()).collect();

        self.client.execute_script(VmExecuteScript {
            senders,
            max_gas_amount: gas.max_gas_amount(),
            gas_unit_price: gas.gas_unit_price(),
            code,
            type_params,
            args,
        })
    }

    /// Asserts that a response is success.
    pub fn assert_success(&self, res: &VmExecuteResponse) {
        match &res.status {
            Some(status) => {
                match &status.error {
                    None => {
                        // no-op
                    }
                    Some(error) => {
                        panic!("Error:[{:?}]", error);
                    }
                }
            }
            None => {
                panic!("Unexpected status [None]");
            }
        }
    }

    /// Merge execution result.
    pub fn merge_result(&self, exec_resp: &VmExecuteResponse) {
        exec_resp.write_set.iter().for_each(|value| {
            let path = value.path.as_ref().unwrap();
            let path = AccessPath::new(
                AccountAddress::try_from(path.address.clone()).unwrap(),
                path.path.clone(),
            );
            match value.r#type {
                0 /*Value*/ => {
                    self.data_source.insert(path, value.value.clone())
                }
                1 /*Deletion*/ => {
                    self.data_source.delete(path);
                }
                _ => unreachable!(),
            }
        });
    }

    /// Returns mock data source.
    pub fn data_source(&self) -> &MockDataSource {
        &self.data_source
    }
}

/// Returns gas meta.
pub fn gas_meta() -> Gas {
    Gas::new(500_000, 1).unwrap()
}

/// Create a new account address from hex string.
pub fn account(addr: &str) -> AccountAddress {
    AccountAddress::from_hex_literal(addr).unwrap()
}
