use anyhow::Result;
use libra::prelude::*;
use crate::{tonic, api};
use tonic::{Request, Response, Status};
use api::grpc::vm_grpc::vm_compiler_server::VmCompiler;
use api::grpc::vm_grpc::vm_multiple_sources_compiler_server::VmMultipleSourcesCompiler;
use api::grpc::vm_grpc::{
    SourceFile, CompilationResult, SourceFiles, MultipleCompilationResult, CompiledUnit,
};
use std::convert::TryFrom;
use compiler::Compiler;
use info::metrics::meter::ScopeMeter;
use info::metrics::execution::ExecutionResult;
use std::collections::HashMap;

/// Compilation service.
#[derive(Clone)]
pub struct CompilerService<C>
where
    C: RemoteCache + Clone + Send + Sync + 'static,
{
    compiler: Compiler<C>,
}

impl<C> CompilerService<C>
where
    C: RemoteCache + Clone + Send + Sync + 'static,
{
    /// Create a new compiler service with the given compiler.
    pub fn new(compiler: Compiler<C>) -> Self {
        CompilerService { compiler }
    }
}

/// Convert address from bytes.
fn convert_address(addr: &[u8]) -> Result<AccountAddress, Status> {
    AccountAddress::try_from(addr).map_err(|err| Status::invalid_argument(err.to_string()))
}

impl<C> CompilerService<C>
where
    C: RemoteCache + Clone + Send + Sync + 'static,
{
    /// Compile source code.
    async fn compile(
        &self,
        request: Request<SourceFile>,
    ) -> Result<Result<Vec<u8>, String>, Status> {
        let source_file_data = request.into_inner();
        let address = convert_address(&source_file_data.address)?;
        let mut source = HashMap::with_capacity(1);
        source.insert("source".to_owned(), source_file_data.text);
        Ok(self
            .compiler
            .compile_source_map(source, Some(address))
            .map_err(|err| err.to_string())
            .and_then(|bytecode_map| {
                if bytecode_map.len() > 1 {
                    Err("Unsupported multiple modules file.".to_owned())
                } else if let Some((_, bytecode)) = bytecode_map.into_iter().next() {
                    Ok(bytecode)
                } else {
                    Err("".to_owned())
                }
            }))
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
impl<C> VmCompiler for CompilerService<C>
where
    C: RemoteCache + Clone + Send + Sync + 'static,
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
impl<C> VmMultipleSourcesCompiler for CompilerService<C>
where
    C: RemoteCache + Clone + Send + Sync + 'static,
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
