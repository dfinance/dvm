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
use runtime::move_vm::ExecutionMeta;
use libra::prelude::*;
use std::convert::TryFrom;
use crate::grpc_client::Client;
use data_source::MockDataSource;
use lang::{
    stdlib::{build_std, zero_std},
};
use compiler::Compiler;
pub use genesis::genesis_write_set;
use anyhow::Error;
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
    pub fn publish_module(&self, code: &str, meta: ExecutionMeta) -> VmExecuteResponse {
        let module = self.compiler.compile(code, Some(meta.sender)).unwrap();
        self.client.publish_module(VmPublishModule {
            address: meta.sender.to_vec(),
            max_gas_amount: meta.max_gas_amount,
            gas_unit_price: meta.gas_unit_price,
            code: module,
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

    /// Execute script.
    pub fn execute_script(
        &self,
        code: &str,
        meta: ExecutionMeta,
        args: Vec<VmArgs>,
        type_params: Vec<StructIdent>,
    ) -> VmExecuteResponse {
        let code = self.compiler.compile(code, Some(meta.sender)).unwrap();

        self.client.execute_script(VmExecuteScript {
            address: meta.sender.to_vec(),
            max_gas_amount: meta.max_gas_amount,
            gas_unit_price: meta.gas_unit_price,
            code,
            type_params,
            args,
        })
    }

    /// Asserts that a response is success.
    pub fn assert_success(&self, res: &VmExecuteResponse) {
        if res.status == 0
            || res
                .status_struct
                .as_ref()
                .map(|v| v.major_status != 4001)
                .unwrap_or(false)
        {
            panic!("Error:[{:?}]", res.status_struct);
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

impl StateView for TestKit {
    fn get(&self, access_path: &AccessPath) -> Result<Option<Vec<u8>>, Error> {
        StateView::get(&self.data_source, access_path)
    }

    fn multi_get(&self, access_paths: &[AccessPath]) -> Result<Vec<Option<Vec<u8>>>, Error> {
        self.data_source.multi_get(access_paths)
    }

    fn is_genesis(&self) -> bool {
        self.data_source.is_genesis()
    }
}

/// Returns execution meta with given address.
pub fn meta(addr: &AccountAddress) -> ExecutionMeta {
    ExecutionMeta::new(500_000, 1, *addr)
}

/// Create a new account address from hex string.
pub fn account(addr: &str) -> AccountAddress {
    AccountAddress::from_hex_literal(addr).unwrap()
}
