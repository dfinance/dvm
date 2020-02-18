use anyhow::Result;
use bytecode_verifier::VerifiedModule;
use futures::lock::Mutex;
use libra_types::access_path::AccessPath;
use libra_types::account_address::AccountAddress;
use move_ir_types::ast::ModuleIdent;
use tonic::{Request, Response, Status};
use tonic::transport::Channel;
use vm::CompiledModule;

use crate::compiled_protos::ds_grpc::{DsAccessPath, DsRawResponse};
use crate::compiled_protos::ds_grpc::ds_service_client::DsServiceClient;
use crate::compiled_protos::vm_grpc::{CompilationResult, MvIrSourceFile};
use crate::compiled_protos::vm_grpc::vm_compiler_server::VmCompiler;

pub fn extract_imports(source_text: &str, is_module: bool) -> Result<Vec<AccessPath>> {
    let imports = if is_module {
        ir_to_bytecode::parser::parse_module(source_text)?.imports
    } else {
        ir_to_bytecode::parser::parse_script(source_text)?.imports
    };
    let mut imported_modules = vec![];
    for import in imports {
        if let ModuleIdent::Qualified(module_ident) = import.ident {
            imported_modules.push(AccessPath::new(
                module_ident.address,
                module_ident.name.to_string().into_bytes(),
            ));
        }
    }
    Ok(imported_modules)
}

#[tonic::async_trait]
pub trait DsClient {
    async fn resolve_ds_path(
        &mut self,
        request: tonic::Request<AccessPath>,
    ) -> Result<tonic::Response<DsRawResponse>, tonic::Status>;
}

#[tonic::async_trait]
impl DsClient for DsServiceClient<Channel> {
    async fn resolve_ds_path(
        &mut self,
        request: Request<AccessPath>,
    ) -> Result<Response<DsRawResponse>, Status> {
        let ds_access_path: DsAccessPath = request.into_inner().into();
        let request = Request::new(ds_access_path);
        self.get_raw(request).await
    }
}

pub fn new_compilation_result(bytecode: Vec<u8>) -> CompilationResult {
    CompilationResult {
        bytecode,
        errors: vec![],
    }
}

pub fn new_error_compilation_result(errors: Vec<String>) -> CompilationResult {
    CompilationResult {
        bytecode: vec![],
        errors,
    }
}

pub struct CompilerService {
    ds_client: Mutex<Box<dyn DsClient + Send + Sync>>,
}

impl CompilerService {
    pub fn new(ds_client: Box<dyn DsClient + Send + Sync>) -> Self {
        CompilerService {
            ds_client: Mutex::new(ds_client),
        }
    }
}

impl CompilerService {
    async fn inner_compile(
        &self,
        request: Request<MvIrSourceFile>,
    ) -> Result<Result<Vec<u8>, Vec<String>>, Status> {
        let source_file_data = request.into_inner();

        let source_text = source_file_data.text;
        let is_module = source_file_data.is_module;

        let imports = match extract_imports(&source_text, is_module) {
            Ok(imports) => imports,
            Err(err) => {
                let errors = vec![err.to_string()];
                return Ok(Err(errors));
            }
        };

        let mut deps: Vec<VerifiedModule> = vec![];
        let mut dependency_errors: Vec<String> = vec![];
        for import_access_path in imports {
            let mut client = self.ds_client.lock().await;
            let response = client
                .resolve_ds_path(Request::new(import_access_path))
                .await;
            if let Err(status) = response {
                return Err(Status::unavailable(format!(
                    "DS server request failed with {}",
                    status.to_string()
                )));
            }
            let ds_response = response.unwrap().into_inner();
            match ds_response.error_code {
                0 => {
                    let resolved_dep_module = CompiledModule::deserialize(&ds_response.blob)
                        .expect("Module deserialization failed");
                    deps.push(VerifiedModule::new(resolved_dep_module).unwrap());
                }
                // BadRequest, should not happen
                1 => panic!("DS returned BAD_REQUEST"),
                // NoData, compiler error: cannot resolve a dependency
                2 => {
                    dependency_errors.push(String::from_utf8(ds_response.error_message).unwrap());
                }
                _ => panic!("DS returned invalid ErrorCode"),
            };
        }
        if !dependency_errors.is_empty() {
            return Ok(Err(dependency_errors));
        }

        let address_lit = match String::from_utf8(source_file_data.address.to_vec()) {
            Ok(address) => address,
            Err(_) => return Err(Status::invalid_argument("Address is not a valid utf8")),
        };
        let account_address = AccountAddress::from_hex_literal(&address_lit).unwrap();

        let mut compiler = compiler::Compiler::default();
        compiler.skip_stdlib_deps = true;
        compiler.extra_deps = deps;
        compiler.address = account_address;

        let mut compiled_bytecode = vec![];
        if is_module {
            compiler
                .into_compiled_module(&source_text)
                .unwrap()
                .serialize(&mut compiled_bytecode)
                .unwrap()
        } else {
            compiler
                .into_compiled_program(&source_text)
                .unwrap()
                .script
                .serialize(&mut compiled_bytecode)
                .unwrap()
        };
        Ok(Ok(compiled_bytecode))
    }
}

#[tonic::async_trait]
impl VmCompiler for CompilerService {
    async fn compile(
        &self,
        request: Request<MvIrSourceFile>,
    ) -> Result<Response<CompilationResult>, Status> {
        let res = self.inner_compile(request).await?;
        match res {
            Ok(bytecode) => Ok(Response::new(new_compilation_result(bytecode))),
            Err(errors) => Ok(Response::new(new_error_compilation_result(errors))),
        }
    }
}
