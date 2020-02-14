mod grpc_client;
mod grpc_server;

pub use self::{
    compiler::{Lang, Compiler},
    grpc_client::Client,
};
pub use self::{grpc_client::Client};
use crate::grpc::*;
pub use grpc_server::{Server, Signal};
use std::sync::{Mutex, Arc};
use std::ops::Range;
use crate::ds::MockDataSource;
use crate::vm::ExecutionMeta;
use tonic::Request;
use libra_types::transaction::{TransactionArgument, parse_as_transaction_argument};
use libra_types::access_path::AccessPath;
use libra_types::account_address::AccountAddress;
use std::convert::TryFrom;
use crate::compiled_protos::vm_grpc::{
    VmExecuteRequest, VmContract, VmExecuteResponses, VmArgs, VmValue,
};
use crate::vm::compiler::{Compiler, Lang};

pub const PORT_RANGE: Range<u32> = 3000..5000;

pub type ArcMut<T> = Arc<Mutex<T>>;

pub struct TestKit {
    data_source: MockDataSource,
    client: Client,
    _server: Server,
    compiler: Box<dyn Compiler>,
}

impl TestKit {
    pub fn new(lang: Lang) -> TestKit {
        let data_source = MockDataSource::new(Lang::MvIr);
        let server = Server::new(data_source.clone());
        let client = Client::new(server.port()).unwrap_or_else(|_| {
            panic!(
                "Client couldn't connect to the server at http://localhost:{}",
                server.port()
            )
        });
        TestKit {
            data_source,
            _server: server,
            compiler: lang.compiler(),
            client,
        }
    }

    pub fn publish_module(&self, code: &str, meta: ExecutionMeta) -> VmExecuteResponses {
        let module = self
            .compiler
            .build_module(code, &meta.sender, false)
            .unwrap();
        let request = Request::new(VmExecuteRequest {
            contracts: vec![VmContract {
                address: meta.sender.to_vec(),
                max_gas_amount: meta.max_gas_amount,
                gas_unit_price: meta.gas_unit_price,
                code: module,
                contract_type: 0, //Module
                args: vec![],
            }],
            options: 0,
        });
        self.client.perform_request(request)
    }

    pub fn execute_script(
        &self,
        code: &str,
        meta: ExecutionMeta,
        args: &[&str],
    ) -> VmExecuteResponses {
        let code = self
            .compiler
            .build_script(code, &meta.sender, false)
            .unwrap();

        let args = parse_args(args)
            .into_iter()
            .map(|arg| match arg {
                TransactionArgument::Bool(val) => (0, val.to_string()),
                TransactionArgument::U64(val) => (1, val.to_string()),
                TransactionArgument::ByteArray(val) => (2, val.to_string()),
                TransactionArgument::Address(val) => (3, addr(&val)),
            })
            .map(|(value_type, value)| VmArgs {
                r#type: value_type,
                value,
            })
            .collect();

        let request = Request::new(VmExecuteRequest {
            contracts: vec![VmContract {
                address: meta.sender.to_vec(),
                max_gas_amount: meta.max_gas_amount,
                gas_unit_price: meta.gas_unit_price,
                code,
                contract_type: 1, //Script
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
            .filter(|exec| exec.status == 0 /*Discard*/)
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
