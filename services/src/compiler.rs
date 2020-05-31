use anyhow::Result;
use libra::libra_types;
use libra_types::account_address::AccountAddress;
use crate::{tonic, api};
use tonic::{Request, Response, Status};

use libra::libra_state_view::StateView;
use api::grpc::vm_grpc::vm_compiler_server::VmCompiler;
use api::grpc::vm_grpc::vm_multiple_sources_compiler_server::VmMultipleSourcesCompiler;
use api::grpc::vm_grpc::{
    SourceFile, CompilationResult, SourceFiles, MultipleCompilationResult, CompiledUnit,
};
use std::convert::TryFrom;
use compiler::Compiler;
use info::metrics::meter::ScopeMeter;
use info::metrics::live_time::ExecutionResult;

/// Compilation service.
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
    /// Create a new compiler service with the given compiler.
    pub fn new(compiler: Compiler<S>) -> Self {
        CompilerService { compiler }
    }
}

/// Convert address from bytes.
fn convert_address(addr: &[u8]) -> Result<AccountAddress, Status> {
    AccountAddress::try_from(addr).map_err(|err| Status::invalid_argument(err.to_string()))
}

impl<S> CompilerService<S>
where
    S: StateView + Clone + Send + Sync + 'static,
{
    /// Compile source code.
    async fn compile(
        &self,
        request: Request<SourceFile>,
    ) -> Result<Result<Vec<u8>, String>, Status> {
        let source_file_data = request.into_inner();
        let address = convert_address(&source_file_data.address)?;
        Ok(self
            .compiler
            .compile(&source_file_data.text, Some(address))
            .map_err(|err| err.to_string()))
    }

    /// Compiler source codes.
    async fn multiple_source_compile(
        &self,
        request: Request<SourceFiles>,
    ) -> Result<Result<Vec<CompiledUnit>, String>, Status> {
        let request = request.into_inner();
        let address = convert_address(&request.address)?;
        let source_map = request
            .units
            .into_iter()
            .map(|unit| (unit.name, unit.text))
            .collect();

        Ok(self
            .compiler
            .compile_source_map(source_map, Some(address))
            .map_err(|err| err.to_string())
            .map(|map| {
                map.into_iter()
                    .map(|(name, bytecode)| CompiledUnit { name, bytecode })
                    .collect()
            }))
    }
}

#[tonic::async_trait]
impl<S> VmCompiler for CompilerService<S>
where
    S: StateView + Clone + Send + Sync + 'static,
{
    /// Compile source code.
    async fn compile(
        &self,
        request: Request<SourceFile>,
    ) -> Result<Response<CompilationResult>, Status> {
        let mut meter = ScopeMeter::new("compile");
        match self.compile(request).await {
            Ok(Ok(bytecode)) => {
                meter.set_result(ExecutionResult::new(true, 200, 0));
                Ok(Response::new(CompilationResult::with_bytecode(bytecode)))
            }
            Ok(Err(errors)) => {
                meter.set_result(ExecutionResult::new(false, 400, 0));
                Ok(Response::new(CompilationResult::with_errors(vec![errors])))
            }
            Err(status) => {
                meter.set_result(ExecutionResult::new(false, 500, 0));
                Err(status)
            }
        }
    }
}

#[tonic::async_trait]
impl<S> VmMultipleSourcesCompiler for CompilerService<S>
where
    S: StateView + Clone + Send + Sync + 'static,
{
    /// Compiler source codes.
    async fn compile(
        &self,
        request: Request<SourceFiles>,
    ) -> Result<Response<MultipleCompilationResult>, Status> {
        let mut meter = ScopeMeter::new("multiple_compile");

        match self.multiple_source_compile(request).await {
            Ok(Ok(units)) => {
                meter.set_result(ExecutionResult::new(true, 200, 0));
                Ok(Response::new(MultipleCompilationResult {
                    units,
                    errors: vec![],
                }))
            }
            Ok(Err(errors)) => {
                meter.set_result(ExecutionResult::new(false, 400, 0));
                Ok(Response::new(MultipleCompilationResult {
                    units: vec![],
                    errors: vec![errors],
                }))
            }
            Err(status) => {
                meter.set_result(ExecutionResult::new(false, 500, 0));
                Err(status)
            }
        }
    }
}
