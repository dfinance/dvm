mod grpc_client;
mod grpc_server;

pub use grpc_server::{Server, Signal};
use std::sync::{Mutex, Arc};
use std::ops::Range;
use crate::ds::MockDataSource;
use crate::vm::{ExecutionMeta, bech32_utils};
use tonic::Request;
use libra_types::transaction::{TransactionArgument, parse_as_transaction_argument};
use libra_types::access_path::AccessPath;
use libra_types::account_address::AccountAddress;
use std::convert::TryFrom;
use crate::compiled_protos::vm_grpc::{
    VmExecuteRequest, VmContract, VmExecuteResponses, VmArgs, VmValue, ContractType,
};
use crate::vm::compiler::{Compiler, Lang};
use crate::test_kit::grpc_client::Client;
use crate::vm::stdlib::{move_std, build_std_with_compiler, mvir_std, Stdlib};
use vm::CompiledModule;

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

        let compiler = lang.compiler();
        let std = match &lang {
            Lang::Move => move_std(),
            Lang::MvIr => mvir_std(),
        };

        // TODO remove it when the compiler can take modules from ds.
        build_std_with_compiler(Stdlib { modules: std, lang }, compiler.as_ref()).unwrap();

        TestKit {
            data_source,
            _server: server,
            compiler,
            client,
        }
    }

    pub fn publish_module(&self, code: &str, meta: ExecutionMeta) -> VmExecuteResponses {
        let module = self
            .compiler
            .build_module(code, &meta.sender, true)
            .unwrap();
        let sender_as_bech32 = bech32_utils::libra_into_bech32(&addr(&meta.sender)).unwrap();
        let request = Request::new(VmExecuteRequest {
            contracts: vec![VmContract {
                address: sender_as_bech32,
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
            .build_module(code, &AccountAddress::default(), true)
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
        let code = self
            .compiler
            .build_script(code, &meta.sender, true)
            .unwrap();

        let libra_address = addr(&meta.sender);
        let bech32_sender_address = bech32_utils::libra_into_bech32(&libra_address)
            .expect("Cannot convert to bech32 address");

        let request = Request::new(VmExecuteRequest {
            contracts: vec![VmContract {
                address: bech32_sender_address,
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
