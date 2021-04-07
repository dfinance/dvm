use anyhow::Result;
use libra::prelude::*;
use crate::{tonic, api};
use tonic::{Request, Response, Status};

use api::grpc::dvm_compiler_server::DvmCompiler;
use api::grpc::{CompilationResult, SourceFiles, CompiledUnit};
use std::convert::TryFrom;
use compiler::Compiler;
use info::metrics::meter::ScopeMeter;
use info::metrics::execution::ExecutionResult;

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
    /// Compiler source codes.
    pub async fn compile(
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
impl<C> DvmCompiler for CompilerService<C>
where
    C: RemoteCache + Clone + Send + Sync + 'static,
{
    /// Compiler source codes.
    async fn compile(
        &self,
        request: Request<SourceFiles>,
    ) -> Result<Response<CompilationResult>, Status> {
        let mut meter = ScopeMeter::new("multiple_compile");

        match self.compile(request).await {
            Ok(Ok(units)) => {
                meter.set_result(ExecutionResult::new(true, 200, 0));
                Ok(Response::new(CompilationResult {
                    units,
                    errors: vec![],
                }))
            }
            Ok(Err(errors)) => {
                meter.set_result(ExecutionResult::new(false, 400, 0));
                Ok(Response::new(CompilationResult {
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
