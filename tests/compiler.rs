use libra::{libra_types, vm};
use libra_types::account_address::AccountAddress;
use vm::access::ScriptAccess;
use vm::CompiledModule;
use vm::file_format::{Bytecode, ModuleHandleIndex};
use vm::file_format::CompiledScript;

use dvm_api::tonic;
use tonic::{Request, Response, Status};

use dvm::compiled_protos::vm_grpc::{CompilationResult, ContractType, MvIrSourceFile};
use dvm::compiled_protos::vm_grpc::vm_compiler_server::VmCompiler;
use dvm::services::compiler::CompilerService;
use lang::{compiler::Compiler, stdlib::build_std};
use data_source::MockDataSource;

fn new_source_file(source: &str, r#type: ContractType, address: &str) -> MvIrSourceFile {
    MvIrSourceFile {
        text: source.to_string(),
        r#type: r#type as i32,
        address: address.to_string().into_bytes(),
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

    let compiler = Compiler::new(MockDataSource::with_write_set(build_std()));
    let compiler_service = CompilerService::new(compiler);
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

    CompiledModule::deserialize(&compilation_result.bytecode[..]).unwrap();
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
    let source_file_request = new_source_file_request(source_text, ContractType::Script);

    let compiler = Compiler::new(MockDataSource::with_write_set(build_std()));
    let compiler_service = CompilerService::new(compiler);
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
            import 0x0.Coin;
            main() {
               return;
            }
        ";

    let source_file_request = new_source_file_request(source_text, ContractType::Script);

    let compiler = Compiler::new(MockDataSource::with_write_set(build_std()));
    let compiler_service = CompilerService::new(compiler);
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
        r#"Module with path [ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000000, name: Identifier("Coin") }] not found"#
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

    let ds = MockDataSource::with_write_set(build_std());
    let compiler = Compiler::new(ds.clone());
    let hash = compiler
        .compile(
            "\
        module Hash {}
    ",
            &libra_address,
        )
        .unwrap();
    ds.publish_module(hash).unwrap();

    let compiler_service = CompilerService::new(compiler);
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

    let compiler = Compiler::new(MockDataSource::with_write_set(build_std()));
    let compiler_service = CompilerService::new(compiler);
    let error_status = compiler_service.compile(request).await.unwrap_err();
    assert_eq!(error_status.message(), "Address is not a valid bech32");
}

#[tokio::test]
async fn test_compilation_error_on_expected_an_expression_term() {
    let source_text = r#"
            fun main() {
                let a: u128;
                let a: bytearray;
                return;
            }
        "#;
    let compilation_result = compile_source_file(source_text, ContractType::Script)
        .await
        .unwrap()
        .into_inner();
    assert!(compilation_result.errors[0].contains("Expected an expression term"));
}
