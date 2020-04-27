use anyhow::Result;
use libra::libra_types;
use libra_types::account_address::AccountAddress;
use dvm_api::tonic;
use tonic::{Request, Response, Status};

use lang::{compiler::Compiler, banch32::bech32_into_libra};
use libra::libra_state_view::StateView;

use dvm_api::grpc::vm_grpc::vm_compiler_server::VmCompiler;
use dvm_api::grpc::vm_grpc::{MvIrSourceFile, CompilationResult};

pub struct CompilerService<S>
where
    S: StateView + Clone + Send + Sync + 'static,
{
    compiler: Compiler<S>,
}

impl<S> CompilerService<S>
where
    S: StateView + Clone + Send + Sync + 'static,
{
    pub fn new(compiler: Compiler<S>) -> Self {
        CompilerService { compiler }
    }
}

fn convert_address(addr: &[u8]) -> Result<AccountAddress, Status> {
    std::str::from_utf8(&addr)
        .map_err(|_| Status::invalid_argument("Address is not a valid utf8"))
        .and_then(|address| {
            bech32_into_libra(address)
                .map_err(|_| Status::invalid_argument("Address is not a valid bech32"))
        })
        .and_then(|address| Ok(format!("0x{}", address)))
        .and_then(|address| {
            AccountAddress::from_hex_literal(&address)
                .map_err(|_| Status::invalid_argument("Address is not valid"))
        })
}

impl<S> CompilerService<S>
where
    S: StateView + Clone + Send + Sync + 'static,
{
    async fn inner_compile(
        &self,
        request: Request<MvIrSourceFile>,
    ) -> Result<Result<Vec<u8>, String>, Status> {
        let source_file_data = request.into_inner();
        let address = convert_address(&source_file_data.address)?;
        Ok(self
            .compiler
            .compile(&source_file_data.text, &address)
            .map_err(|err| err.to_string()))
    }
}

#[tonic::async_trait]
impl<S> VmCompiler for CompilerService<S>
where
    S: StateView + Clone + Send + Sync + 'static,
{
    async fn compile(
        &self,
        request: Request<MvIrSourceFile>,
    ) -> Result<Response<CompilationResult>, Status> {
        let res = self.inner_compile(request).await?;
        match res {
            Ok(bytecode) => Ok(Response::new(CompilationResult::with_bytecode(bytecode))),
            Err(errors) => Ok(Response::new(CompilationResult::with_errors(vec![errors]))),
        }
    }
}
