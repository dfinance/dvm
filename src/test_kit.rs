use crate::ds::MockDataSource;
use crate::service::MoveVmService;
use tonic::transport::Server;
use std::thread;
use crate::grpc::{
    vm_service_server::*, *,
    vm_service_client::*
};
use std::{
    future::Future,
};
use tokio::{
    runtime::Runtime,
};
use futures::task::{Context, Poll};
use std::sync::{Mutex, Arc};

use tonic::codegen::Pin;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::ops::Range;
use std::sync::atomic::{AtomicU32, Ordering};
use std::io::{
    ErrorKind,
    Error as IoError,
};
use std::convert::TryFrom;
use libra_types::access_path::AccessPath;
use libra_types::account_address::AccountAddress;
use crate::move_lang::{ExecutionMeta, build_with_deps, Code};
use libra_types::transaction::TransactionArgument;
use compiler::Compiler as MvIrCompiler;
use move_lang::to_bytecode::translate::CompiledUnit;
use bytecode_verifier::VerifiedModule;

const PORT_RANGE: Range<u32> = 3000..5000;

pub struct TestKit {
    data_source: MockDataSource,
    signal: ShutdownSignal,
    port: Arc<AtomicU32>,
    compiler: Box<dyn Compiler>,
}

impl TestKit {
    pub fn new(lang: Lang) -> TestKit {
        let data_source = MockDataSource::default();
        let signal = ShutdownSignal::new();
        let port = Arc::new(AtomicU32::new(0));

        let service_port = port.clone();
        let service_signal = signal.clone();
        let service_data_source = data_source.clone();
        thread::spawn(move || {
            let mut rt = Runtime::new().unwrap();
            rt.block_on(async {
                for port in PORT_RANGE {
                    service_port.store(port, Ordering::Relaxed);
                    let service_res = Server::builder()
                        .add_service(VmServiceServer::new(MoveVmService::new(Box::new(service_data_source.clone()))))
                        .serve_with_shutdown(format!("127.0.0.1:{}", port).parse().unwrap(), service_signal.clone())
                        .await;
                    match service_res {
                        Ok(_) => break,
                        Err(_) => {
                            if IoError::last_os_error().kind() == ErrorKind::AddrInUse {
                                continue;
                            }
                            break;
                        }
                    }
                }
            });
        });

        TestKit {
            data_source,
            signal,
            port,
            compiler: lang.compiler(),
        }
    }

    pub fn publish_module(&self, code: &str, meta: ExecutionMeta) -> VmExecuteResponse {
        let module = self.compiler.build_module(code, &meta.sender);

        let request = tonic::Request::new(VmExecuteRequest {
            contracts: vec![VmContract {
                address: meta.sender.to_vec(),
                max_gas_amount: meta.max_gas_amount,
                gas_unit_price: meta.gas_unit_price,
                code: module,
                contract_type: 0, //Module
                args: vec![]
            }],
            options: 0, // u64
        });

        todo!()
    }

    pub fn execute_script(&self, code: &str, meta: ExecutionMeta, args: Vec<TransactionArgument>) -> VmExecuteResponse {
        let script = self.compiler.build_script(code, &meta.sender);

        let request = tonic::Request::new(VmExecuteRequest {
            contracts: vec![VmContract {
                address: meta.sender.to_vec(),
                max_gas_amount: meta.max_gas_amount,
                gas_unit_price: meta.gas_unit_price,
                code: script,
                contract_type: 1, //Script
                args: vec![]
            }],
            options: 0, // u64
        });

        todo!()
    }

    pub fn assert_success(&self, res: VmExecuteResponse) {
        todo!()
    }


    pub fn merge_write_sets(&self, exec_resp: &VmExecuteResponses) {
        exec_resp.executions.iter()
            .for_each(|exec| self.merge_write_set(&exec.write_set));
    }

    fn merge_write_set(&self, ws: &Vec<VmValue>) {
        ws.iter()
            .for_each(|value| {
                let path = value.path.as_ref().unwrap();
                let path = AccessPath::new(AccountAddress::try_from(path.address.clone()).unwrap(), path.path.clone());
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

impl Drop for TestKit {
    fn drop(&mut self) {
        self.signal.shutdown();
    }
}

#[derive(Clone)]
struct ShutdownSignal {
    tx: Sender<()>,
    rx: Arc<Mutex<Receiver<()>>>,
}

impl ShutdownSignal {
    pub fn new() -> ShutdownSignal {
        let (tx, rx) = channel();

        ShutdownSignal {
            tx,
            rx: Arc::new(Mutex::new(rx)),
        }
    }

    pub fn shutdown(&self) {
        self.tx.send(()).unwrap();
    }
}

impl Future for ShutdownSignal {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.rx.lock().unwrap().recv().unwrap();
        Poll::Ready(())
    }
}

pub enum Lang {
    Move,
    MvIr,
}

impl Lang {
    pub fn compiler(&self) -> Box<dyn Compiler> {
        match self {
            Lang::Move => Box::new(Move::new()),
            Lang::MvIr => Box::new(MvIr::new()),
        }
    }
}

trait Compiler {
    fn build_module(&self, code: &str, address: &AccountAddress) -> Vec<u8>;
    fn build_script(&self, code: &str, address: &AccountAddress) -> Vec<u8>;
}

pub struct Move {
    cache: Mutex<Vec<String>>,
}

impl Move {
    pub fn new() -> Move {
        Move {
            cache: Mutex::new(vec![])
        }
    }
}

impl Compiler for Move {
    fn build_module(&self, code: &str, address: &AccountAddress) -> Vec<u8> {
        let mut cache = self.cache.lock().unwrap();
        let deps = cache.iter()
            .map(|dep| Code::module("dep", dep))
            .collect();
        let module = build_with_deps(Code::module("source", code), deps, address).unwrap();
        module.serialize()
    }

    fn build_script(&self, code: &str, address: &AccountAddress) -> Vec<u8> {
        let cache = self.cache.lock().unwrap();
        let deps = cache.iter()
            .map(|dep| Code::module("dep", dep))
            .collect();
        let module = build_with_deps(Code::script(code), deps, address).unwrap();
        module.serialize()
    }
}

pub struct MvIr {
    cache: Mutex<Vec<VerifiedModule>>,
}

impl MvIr {
    pub fn new() -> MvIr {
        MvIr {
            cache: Mutex::new(vec![])
        }
    }
}

impl Compiler for MvIr {
    fn build_module(&self, code: &str, address: &AccountAddress) -> Vec<u8> {
        let mut cache = self.cache.lock().unwrap();
        let mut compiler = MvIrCompiler::default();
        compiler.extra_deps = cache.clone();
        compiler.address = address.clone();
        let module = compiler.into_compiled_module(code).unwrap();
        let mut buff = Vec::new();
        module.serialize(&mut buff).unwrap();
        cache.push(VerifiedModule::new(module).unwrap());
        buff
    }

    fn build_script(&self, code: &str, address: &AccountAddress) -> Vec<u8> {
        let cache = self.cache.lock().unwrap();
        let mut compiler = MvIrCompiler::default();
        compiler.extra_deps = cache.clone();
        compiler.address = address.clone();
        let module = compiler.into_compiled_program(code).unwrap();
        let mut buff = Vec::new();
        module.script.serialize(&mut buff).unwrap();
        buff
    }
}
