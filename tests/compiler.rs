use std::collections::HashMap;

use bytecode_verifier::VerifiedModule;
use libra_types::access_path::AccessPath;
use libra_types::account_address::AccountAddress;
use libra_types::identifier::Identifier;
use libra_types::language_storage::ModuleId;
use maplit::hashmap;
use tonic::{Request, Response, Status};
use vm::access::ScriptAccess;
use vm::CompiledModule;
use vm::file_format::{Bytecode, ModuleHandleIndex};
use vm::file_format::CompiledScript;

use move_vm_in_cosmos::compiled_protos::ds_grpc::ds_raw_response::ErrorCode;
use move_vm_in_cosmos::compiled_protos::ds_grpc::DsRawResponse;
use move_vm_in_cosmos::compiled_protos::vm_grpc::{CompilationResult, ContractType, MvIrSourceFile};
use move_vm_in_cosmos::compiled_protos::vm_grpc::vm_compiler_server::VmCompiler;
use move_vm_in_cosmos::compiler::mvir::{CompilerService, DsClient};
use move_vm_in_cosmos::compiler::test_utils::{new_error_response, new_response};
use move_vm_in_cosmos::vm::Lang;

fn new_source_file(source: &str, r#type: ContractType, address: &str) -> MvIrSourceFile {
    MvIrSourceFile {
        text: source.to_string(),
        r#type: r#type as i32,
        address: address.to_string().into_bytes(),
    }
}

fn hash_module() -> VerifiedModule {
    let hash = Lang::MvIr
        .compiler()
        .build_module(
            include_str!("../stdlib/mvir/hash.mvir"),
            &AccountAddress::default(),
            true,
        )
        .unwrap();
    VerifiedModule::new(CompiledModule::deserialize(&hash).unwrap()).unwrap()
}

#[derive(Default)]
struct DsServiceMock {
    deps: HashMap<AccessPath, VerifiedModule>,
}

impl DsServiceMock {
    #[allow(dead_code)]
    pub fn with_deps(deps: HashMap<(AccountAddress, String), VerifiedModule>) -> Self {
        let deps: HashMap<AccessPath, VerifiedModule> = deps
            .iter()
            .map(|(key, val)| {
                let (address, path) = key;
                let module_ident = Identifier::new(path.clone()).unwrap();
                let module_id = ModuleId::new(*address, module_ident);
                (AccessPath::code_access_path(&module_id), val.clone())
            })
            .collect();
        Self { deps }
    }
}

#[tonic::async_trait]
impl DsClient for DsServiceMock {
    async fn resolve_ds_path(
        &mut self,
        request: Request<AccessPath>,
    ) -> Result<Response<DsRawResponse>, Status> {
        let access_path = request.into_inner();
        let path = &access_path.path;
        assert_eq!(
            path[0], 0,
            "First byte should be 0 as in AccessPath::CODE_TAG"
        );

        let response = match self.deps.get(&access_path) {
            Some(module) => {
                let mut buffer = vec![];
                module.serialize(&mut buffer).unwrap();
                new_response(&buffer[..])
            }
            None => new_error_response(ErrorCode::NoData, format!("'{}' not found", access_path)),
        };
        Ok(response)
    }
}

fn new_source_file_request(source_text: &str, r#type: ContractType) -> Request<MvIrSourceFile> {
    let address = "cosmos1sxqtxa3m0nh5fu2zkyfvh05tll8fmz8tk2e22e";
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
            import 0x0.Hash;
            main() {
               return;
            }
        ";

    let ds_client = DsServiceMock::with_deps(hashmap! {
        (AccountAddress::default(), "Hash".to_string()) => hash_module()
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
        "Hash"
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
    assert_eq!(
        error,
        r#"'AccessPath { address: 0000000000000000000000000000000000000000000000000000000000000000, type: Module, hash: "1ff6fadddda5de4c8c9bc95c5b204a999070f1c90c97b8017d4beb7a55d5fb30", suffix: "" } ' not found"#
    )
}

#[tokio::test]
async fn test_allows_for_bech32_addresses() {
    let source_text = r"
            import cosmos1sxqtxa3m0nh5fu2zkyfvh05tll8fmz8tk2e22e.Hash;
            main() {
               return;
            }
        ";

    let source_file_request = new_source_file_request(source_text, ContractType::Script);

    let libra_address = AccountAddress::from_hex_literal(
        "0x636f736d6f730000000000008180b3763b7cef44f142b112cbbe8bffce9d88eb",
    )
    .unwrap();

    let ds_client = DsServiceMock::with_deps(hashmap! {
        (libra_address, "Hash".to_string()) => hash_module()
    });

    let compiler_service = CompilerService::new(Box::new(ds_client));
    let compilation_result = compiler_service
        .compile(source_file_request)
        .await
        .unwrap()
        .into_inner();
    assert_eq!(
        compilation_result.errors.len(),
        0,
        "{:?}",
        compilation_result.errors
    );
}

#[tokio::test]
async fn test_pass_empty_string_as_address() {
    let source_text = r"
            main() {
                return;
            }
        ";
    let source_file = new_source_file(source_text, ContractType::Script, "");
    let request = Request::new(source_file);

    let compiler_service = CompilerService::new(Box::new(DsServiceMock::default()));
    let error_status = compiler_service.compile(request).await.unwrap_err();
    assert_eq!(error_status.message(), "Address is not a valid bech32");
}

#[tokio::test]
async fn test_compilation_error_on_variable_redefinition() {
    let source_text = r#"
            main() {
                let a: u128;
                let a: bytearray;
                return;
            }
        "#;
    let compilation_result = compile_source_file(source_text, ContractType::Script)
        .await
        .unwrap()
        .into_inner();
    assert_eq!(compilation_result.errors, vec!["variable redefinition a"]);
}
