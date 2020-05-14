mod genesis;
mod grpc_client;
mod grpc_server;

pub use grpc_server::{Server, Signal};
use std::sync::{Mutex, Arc};
use std::ops::Range;
use runtime::move_vm::ExecutionMeta;
use dvm_api::tonic::Request;

use libra::{libra_types, libra_vm};
use libra_types::transaction::{TransactionArgument, parse_as_transaction_argument};
use libra_types::access_path::AccessPath;
use libra_types::account_address::AccountAddress;
use std::convert::TryFrom;
use crate::compiled_protos::vm_grpc::{
    VmExecuteRequest, VmContract, VmExecuteResponses, VmArgs, VmValue, ContractType,
};
use crate::grpc_client::Client;
use libra_vm::CompiledModule;
use libra::libra_state_view::StateView;
use data_source::MockDataSource;
use lang::{
    compiler::Compiler,
    stdlib::{build_std, zero_sdt},
};
pub use genesis::genesis_write_set;
use anyhow::Error;
use libra_types::write_set::WriteSet;

// TODO: [REF] rename to api_grpc
pub mod compiled_protos {
    extern crate dvm_api;

    pub use dvm_api::grpc::*;
}

pub const PORT_RANGE: Range<u32> = 3000..5000;

pub type ArcMut<T> = Arc<Mutex<T>>;

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
    pub fn new() -> TestKit {
        Self::with_genesis(build_std())
    }

    pub fn empty() -> Self {
        Self::with_genesis(zero_sdt())
    }

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

    pub fn publish_module(&self, code: &str, meta: ExecutionMeta) -> VmExecuteResponses {
        let module = self.compiler.compile(code, &meta.sender).unwrap();
        let request = Request::new(VmExecuteRequest {
            contracts: vec![VmContract {
                address: addr(&meta.sender),
                max_gas_amount: meta.max_gas_amount,
                gas_unit_price: meta.gas_unit_price,
                code: module,

                contract_type: ContractType::Module as i32,
                args: vec![],
            }],
            options: 0,
        });
        self.client.perform_request(request)
    }

    pub fn add_std_module(&self, code: &str) {
        let module = self
            .compiler
            .compile(code, &AccountAddress::default())
            .unwrap();

        let id = CompiledModule::deserialize(&module).unwrap().self_id();
        self.data_source.insert((&id).into(), module);
    }

    pub fn execute_script(
        &self,
        code: &str,
        meta: ExecutionMeta,
        args: Vec<VmArgs>,
    ) -> VmExecuteResponses {
        let code = self.compiler.compile(code, &meta.sender).unwrap();

        let request = Request::new(VmExecuteRequest {
            contracts: vec![VmContract {
                address: addr(&meta.sender),
                max_gas_amount: meta.max_gas_amount,
                gas_unit_price: meta.gas_unit_price,
                code,
                contract_type: ContractType::Script as i32,
                args,
            }],
            options: 0,
        });
        self.client.perform_request(request)
    }

    pub fn assert_success(&self, res: &VmExecuteResponses) {
        let errs: Vec<_> = res
            .executions
            .iter()
            .filter(|exec| {
                exec.status == 0 /*Discard*/ || exec.status_struct.as_ref().map(|v| v.major_status != 4001)
                .unwrap_or(false)
            })
            .map(|exec| format!("err: {:?}", exec.status_struct))
            .collect();
        if !errs.is_empty() {
            panic!("Errors:[{}]", errs.join("\n"));
        }
    }

    pub fn merge_result(&self, exec_resp: &VmExecuteResponses) {
        exec_resp
            .executions
            .iter()
            .for_each(|exec| self.merge_write_set(&exec.write_set));
    }

    pub fn data_source(&self) -> &MockDataSource {
        &self.data_source
    }

    fn merge_write_set(&self, ws: &[VmValue]) {
        ws.iter().for_each(|value| {
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
}

impl StateView for TestKit {
    fn get(&self, access_path: &AccessPath) -> Result<Option<Vec<u8>>, Error> {
        self.data_source.get(access_path)
    }

    fn multi_get(&self, access_paths: &[AccessPath]) -> Result<Vec<Option<Vec<u8>>>, Error> {
        self.data_source.multi_get(access_paths)
    }

    fn is_genesis(&self) -> bool {
        self.data_source.is_genesis()
    }
}

pub fn parse_args(args: &[&str]) -> Vec<TransactionArgument> {
    args.iter()
        .map(|arg| parse_as_transaction_argument(arg).unwrap())
        .collect()
}

pub fn meta(addr: &AccountAddress) -> ExecutionMeta {
    ExecutionMeta::new(std::u64::MAX, 1, *addr)
}

pub fn addr(addr: &AccountAddress) -> String {
    format!("0x{}", addr)
}
