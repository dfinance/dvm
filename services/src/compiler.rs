use anyhow::Result;
use libra::libra_types;
use libra_types::account_address::AccountAddress;
use dvm_api::tonic;
use tonic::{Request, Response, Status};

use lang::{compiler::Compiler};
use libra::libra_state_view::StateView;

use dvm_api::grpc::vm_grpc::vm_compiler_server::VmCompiler;
use dvm_api::grpc::vm_grpc::{MvIrSourceFile, CompilationResult};
use std::convert::TryFrom;

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
    AccountAddress::try_from(addr).map_err(|err| Status::invalid_argument(err.to_string()))
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
