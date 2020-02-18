use std::collections::HashMap;

use bytecode_verifier::VerifiedModule;
use libra_types::access_path::AccessPath;
use libra_types::account_address::AccountAddress;
use maplit::hashmap;
use tonic::{Request, Response, Status};
use vm::access::{ModuleAccess, ScriptAccess};
use vm::CompiledModule;
use vm::file_format::{Bytecode, ModuleHandleIndex};
use vm::file_format::CompiledScript;

use move_vm_in_cosmos::compiled_protos::ds_grpc::ds_raw_response::ErrorCode;
use move_vm_in_cosmos::compiled_protos::ds_grpc::DsRawResponse;
use move_vm_in_cosmos::compiled_protos::vm_grpc::{CompilationResult, MvIrSourceFile, ContractType};
use move_vm_in_cosmos::compiled_protos::vm_grpc::vm_compiler_server::VmCompiler;
use move_vm_in_cosmos::compiler::mvir::{CompilerService, DsClient};
use move_vm_in_cosmos::compiler::test_utils::{new_error_response, new_response};

fn new_source_file(source: &str, r#type: ContractType, address: &str) -> MvIrSourceFile {
    MvIrSourceFile {
        text: source.to_string(),
        r#type: r#type as i32,
        address: address.to_string().into_bytes(),
    }
}

#[derive(Default)]
struct DsServiceMock {
    deps: HashMap<AccessPath, VerifiedModule>,
}

impl DsServiceMock {
    #[allow(dead_code)]
    pub fn with_deps(deps: HashMap<AccessPath, VerifiedModule>) -> Self {
        DsServiceMock { deps }
    }
}

#[tonic::async_trait]
impl DsClient for DsServiceMock {
    async fn resolve_ds_path(
        &mut self,
        request: Request<AccessPath>,
    ) -> Result<Response<DsRawResponse>, Status> {
        let access_path = request.into_inner();
        let response = match self.deps.get(&access_path) {
            Some(module) => {
                let mut buffer = vec![];
                module.serialize(&mut buffer).unwrap();
                new_response(&buffer[..])
            }
            None => new_error_response(
                ErrorCode::NoData,
                format!(
                    "No module '{}' found",
                    String::from_utf8(access_path.path).unwrap()
                ),
            ),
        };
        Ok(response)
    }
}

fn new_source_file_request(source_text: &str, r#type: ContractType) -> Request<MvIrSourceFile> {
    let address = format!("0x{}", AccountAddress::random().to_string());
    let source_file = new_source_file(source_text, r#type, &address);
    Request::new(source_file)
}

async fn compile_source_file(
    source_text: &str,
    r#type: ContractType,
) -> Result<Response<CompilationResult>, Status> {
    let source_file_request = new_source_file_request(source_text, r#type);
    let mocked_ds_client = DsServiceMock::default();

    let compiler_service = CompilerService::new(Box::new(mocked_ds_client));
    compiler_service.compile(source_file_request).await
}

#[tokio::test]
async fn test_compile_mvir_module() {
    let source_text = r"
            module M {
                public method() {
                   return;
                }
            }
        ";
    let compilation_result = compile_source_file(source_text, ContractType::Module)
        .await
        .unwrap()
        .into_inner();
    assert!(
        compilation_result.errors.is_empty(),
        "{:?}",
        compilation_result.errors
    );

    let compiled_module = CompiledModule::deserialize(&compilation_result.bytecode[..]).unwrap();
    dbg!(compiled_module);
}

#[tokio::test]
async fn test_compile_mvir_script() {
    let source_text = r"
            main() {
                return;
            }
        ";
    let compilation_result = compile_source_file(source_text, ContractType::Script)
        .await
        .unwrap()
        .into_inner();
    assert!(
        compilation_result.errors.is_empty(),
        "{:?}",
        compilation_result.errors
    );
    let compiled_script = CompiledScript::deserialize(&compilation_result.bytecode[..]).unwrap();
    assert_eq!(compiled_script.main().code.code, vec![Bytecode::Ret]);
}

#[tokio::test]
async fn test_compile_mvir_script_with_dependencies() {
    let source_text = r"
            import 0x0.LibraCoin;
            main() {
               return;
            }
        ";

    let libracoin_access_path = AccessPath::new(
        AccountAddress::default(),
        "LibraCoin".to_string().into_bytes(),
    );
    let coin_module = stdlib::stdlib_modules()
        .iter()
        .find(|module| module.as_inner().name().as_str() == "LibraCoin")
        .unwrap()
        .clone();
    let ds_client = DsServiceMock::with_deps(hashmap! {
        libracoin_access_path => coin_module
    });

    let source_file_request = new_source_file_request(source_text, ContractType::Script);

    let compiler_service = CompilerService::new(Box::new(ds_client));
    let compilation_result = compiler_service
        .compile(source_file_request)
        .await
        .unwrap()
        .into_inner();
    assert!(
        compilation_result.errors.is_empty(),
        "{:?}",
        compilation_result.errors
    );

    let compiled_script = CompiledScript::deserialize(&compilation_result.bytecode[..]).unwrap();
    assert_eq!(compiled_script.main().code.code, vec![Bytecode::Ret]);

    let imported_module_handle = compiled_script.module_handle_at(ModuleHandleIndex::new(1u16));
    assert_eq!(
        compiled_script
            .identifier_at(imported_module_handle.name)
            .to_string(),
        "LibraCoin"
    );
}

#[tokio::test]
async fn test_required_libracoin_dependency_is_not_available() {
    let source_text = r"
            import 0x0.LibraCoin;
            main() {
               return;
            }
        ";

    let source_file_request = new_source_file_request(source_text, ContractType::Script);

    let compiler_service = CompilerService::new(Box::new(DsServiceMock::default()));
    let compilation_result = compiler_service
        .compile(source_file_request)
        .await
        .unwrap()
        .into_inner();
    assert!(compilation_result.bytecode.is_empty());
    assert_eq!(compilation_result.errors.len(), 1);

    let error = compilation_result.errors.get(0).unwrap();
    assert_eq!(error, "No module 'LibraCoin' found")
}
