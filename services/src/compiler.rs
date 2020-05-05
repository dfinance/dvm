use anyhow::Result;
use libra::libra_types;
use libra_types::account_address::AccountAddress;
use dvm_api::tonic;
use tonic::{Request, Response, Status};

use lang::compiler::Compiler;
use libra::libra_state_view::StateView;
use dvm_api::grpc::vm_grpc::vm_compiler_server::VmCompiler;
use dvm_api::grpc::vm_grpc::vm_multiple_sources_compiler_server::VmMultipleSourcesCompiler;
use dvm_api::grpc::vm_grpc::{SourceFile, CompilationResult, SourceFiles, MultipleCompilationResult, CompiledUnit};
use std::convert::TryFrom;

#[derive(Clone)]
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
    async fn compile(
        &self,
        request: Request<SourceFile>,
    ) -> Result<Result<Vec<u8>, String>, Status> {
        let source_file_data = request.into_inner();
        let address = convert_address(&source_file_data.address)?;
        Ok(self
            .compiler
            .compile(&source_file_data.text, &address)
            .map_err(|err| err.to_string()))
    }

    async fn multiple_source_compile(&self, request: Request<SourceFiles>) -> Result<Result<Vec<CompiledUnit>, String>, Status> {
        let request = request.into_inner();
        let address = convert_address(&request.address)?;
        let source_map = request.units.into_iter()
            .map(|unit| (unit.name, unit.text))
            .collect();

        Ok(self.compiler.compile_source_map(source_map, &address)
            .map_err(|err| err.to_string())
            .map(|map|
                map.into_iter()
                    .map(|(name, bytecode)| {
                        CompiledUnit {
                            name,
                            bytecode,
                        }
                    }).collect()
            ))
    }
}

#[tonic::async_trait]
impl<S> VmCompiler for CompilerService<S>
    where
        S: StateView + Clone + Send + Sync + 'static,
{
    async fn compile(
        &self,
        request: Request<SourceFile>,
    ) -> Result<Response<CompilationResult>, Status> {
        let res = self.compile(request).await?;
        match res {
            Ok(bytecode) => Ok(Response::new(CompilationResult::with_bytecode(bytecode))),
            Err(errors) => Ok(Response::new(CompilationResult::with_errors(vec![errors]))),
        }
    }
}

#[tonic::async_trait]
impl<S> VmMultipleSourcesCompiler for CompilerService<S>
    where
        S: StateView + Clone + Send + Sync + 'static,
{
    async fn compile(&self, request: Request<SourceFiles>) -> Result<Response<MultipleCompilationResult>, Status> {
        let compilation_result = self.multiple_source_compile(request).await?;

        let result = match compilation_result {
            Ok(units) => MultipleCompilationResult { units, errors: vec![] },
            Err(errors) => MultipleCompilationResult { units: vec![], errors: vec![errors] },
        };

        Ok(Response::new(result))
    }
}