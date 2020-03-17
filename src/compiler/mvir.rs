use anyhow::Result;
use bytecode_verifier::VerifiedModule;
use futures::lock::Mutex;
use libra_types::access_path::AccessPath;
use libra_types::account_address::AccountAddress;
use libra_types::language_storage::ModuleId;
use move_ir_types::ast::ModuleIdent;
use tonic::{Request, Response, Status};
use tonic::transport::Channel;
use vm::CompiledModule;

use crate::compiled_protos::ds_grpc::{DsAccessPath, DsRawResponse};
use crate::compiled_protos::ds_grpc::ds_raw_response::ErrorCode;
use crate::compiled_protos::ds_grpc::ds_service_client::DsServiceClient;
use crate::compiled_protos::vm_grpc::{CompilationResult, ContractType, MvIrSourceFile};
use crate::compiled_protos::vm_grpc::vm_compiler_server::VmCompiler;
use crate::vm::bech32_utils;
use regex::Regex;

pub fn extract_imports(source_text: &str, is_module: bool) -> Result<Vec<AccessPath>> {
    let imports = if is_module {
        ir_to_bytecode::parser::parse_module(source_text)?.imports
    } else {
        ir_to_bytecode::parser::parse_script(source_text)?.imports
    };
    let mut imported_modules = vec![];
    for import in imports {
        if let ModuleIdent::Qualified(module_ident) = import.ident {
            let module_id = ModuleId::new(module_ident.address, module_ident.name.into_inner());
            let access_path = AccessPath::code_access_path(&module_id);
            imported_modules.push(access_path);
        }
    }
    Ok(imported_modules)
}

pub fn find_and_replace_s_prefixed_strings(source: &str) -> String {
    let mut replaced = source.to_string();
    let regex = Regex::new(r#"s".*""#).unwrap();
    for mat in regex.find_iter(source) {
        let content = &mat.as_str()[2..mat.as_str().len() - 1];
        let hex = hex::encode(&content.bytes().collect::<Vec<u8>>());
        replaced.replace_range(mat.range(), &format!("h\"{}\"", hex));
    }
    replaced
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

impl CompilationResult {
    pub fn with_bytecode(bytecode: Vec<u8>) -> Self {
        CompilationResult {
            bytecode,
            errors: vec![],
        }
    }

    pub fn with_errors(errors: Vec<String>) -> Self {
        CompilationResult {
            bytecode: vec![],
            errors,
        }
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

pub fn compile_mvir(
    source_text: &str,
    sender_address: AccountAddress,
    is_module: bool,
    deps: Vec<VerifiedModule>,
) -> Result<Vec<u8>, Vec<String>> {
    let mut compiler = compiler::Compiler::default();
    compiler.skip_stdlib_deps = true;
    compiler.extra_deps = deps;
    compiler.address = sender_address;

    let mut compiled_bytecode = vec![];
    if is_module {
        let compiled_module = compiler
            .into_compiled_module(source_text)
            .map_err(|err| vec![err.to_string()])?;
        compiled_module
            .serialize(&mut compiled_bytecode)
            .expect("Module serialization failed");
    } else {
        let compiled_script = compiler
            .into_compiled_program(source_text)
            .map_err(|err| vec![err.to_string()])?
            .script;
        compiled_script
            .serialize(&mut compiled_bytecode)
            .expect("Script serialization failed");
    }
    Ok(compiled_bytecode)
}

impl CompilerService {
    async fn inner_compile(
        &self,
        request: Request<MvIrSourceFile>,
    ) -> Result<Result<Vec<u8>, Vec<String>>, Status> {
        let source_file_data = request.into_inner();

        let source_text = bech32_utils::find_and_replace_bech32_addresses(&source_file_data.text);
        let source_text = find_and_replace_s_prefixed_strings(&source_text);
        let is_module = ContractType::from_i32(source_file_data.r#type)
            .expect("Invalid ContractType")
            == ContractType::Module;

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
            let ds_response = response
                .map_err(|status| {
                    Status::unavailable(format!(
                        "DS server request failed with {}",
                        status.to_string()
                    ))
                })?
                .into_inner();
            let error_code =
                ErrorCode::from_i32(ds_response.error_code).expect("DS returned invalid ErrorCode");
            match error_code {
                ErrorCode::None => {
                    let resolved_dep_module = CompiledModule::deserialize(&ds_response.blob)
                        .expect("Module deserialization failed");
                    deps.push(
                        VerifiedModule::new(resolved_dep_module)
                            .expect("Module verification failed"),
                    );
                }
                // should not happen
                ErrorCode::BadRequest => panic!("DS returned BAD_REQUEST"),
                // NoData, compiler error: cannot resolve a dependency
                ErrorCode::NoData => {
                    dependency_errors.push(ds_response.error_message);
                }
            };
        }
        if !dependency_errors.is_empty() {
            return Ok(Err(dependency_errors));
        }

        let bech32_address_lit = match std::str::from_utf8(&source_file_data.address) {
            Ok(address) => address,
            Err(_) => return Err(Status::invalid_argument("Address is not a valid utf8")),
        };
        let address_lit = match bech32_utils::bech32_into_libra(bech32_address_lit) {
            Ok(address) => format!("0x{}", address),
            Err(_) => {
                return Err(Status::invalid_argument("Address is not a valid bech32"));
            }
        };
        let account_address = AccountAddress::from_hex_literal(&address_lit).unwrap();

        let compiled = compile_mvir(&source_text, account_address, is_module, deps);
        Ok(compiled)
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
            Ok(bytecode) => Ok(Response::new(CompilationResult::with_bytecode(bytecode))),
            Err(errors) => Ok(Response::new(CompilationResult::with_errors(errors))),
        }
    }
}
