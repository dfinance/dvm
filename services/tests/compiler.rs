use libra::{libra_types, libra_vm};
use libra_types::account_address::AccountAddress;
use libra_vm::access::ScriptAccess;
use libra_vm::CompiledModule;
use libra_vm::file_format::{Bytecode, ModuleHandleIndex};
use libra_vm::file_format::CompiledScript;

use dvm_api::tonic;
use tonic::{Request, Response, Status};

use lang::{compiler::Compiler, stdlib::zero_sdt};
use data_source::MockDataSource;
use dvm_api::grpc::vm_grpc::{CompilationResult, ContractType, MvIrSourceFile};
use dvm_services::compiler::CompilerService;
use dvm_api::grpc::vm_grpc::vm_compiler_server::VmCompiler;

fn new_source_file(source: &str, r#type: ContractType, address: &str) -> MvIrSourceFile {
    MvIrSourceFile {
        text: source.to_string(),
        r#type: r#type as i32,
        address: address.to_string().into_bytes(),
    }
}

fn new_source_file_request(source_text: &str, r#type: ContractType) -> Request<MvIrSourceFile> {
    let address = "df1pfk58n7j62uenmam7f9ncu6qnffc2q5dpwuute";
    let source_file = new_source_file(source_text, r#type, &address);
    Request::new(source_file)
}

async fn compile_source_file(
    source_text: &str,
    r#type: ContractType,
) -> Result<Response<CompilationResult>, Status> {
    let source_file_request = new_source_file_request(source_text, r#type);

    let compiler = Compiler::new(MockDataSource::with_write_set(zero_sdt()));
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

    let ds = MockDataSource::with_write_set(zero_sdt());
    let compiler = Compiler::new(ds.clone());
    let hash = compiler
        .compile("module Hash {}", &AccountAddress::default())
        .unwrap();
    ds.publish_module(hash).unwrap();

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

    let compiler = Compiler::new(MockDataSource::with_write_set(zero_sdt()));
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
        r#"Module with path [ModuleId { address: 000000000000000000000000000000000000000000000000, name: Identifier("Coin") }] not found"#
    )
}

#[tokio::test]
async fn test_allows_for_bech32_addresses() {
    let source_text = r"
            import df1pfk58n7j62uenmam7f9ncu6qnffc2q5dpwuute.Hash;
            main() {
               return;
            }
        ";

    let source_file_request = new_source_file_request(source_text, ContractType::Script);

    let libra_address =
        AccountAddress::from_hex_literal("0x646600000a6d43cfd2d2b999efbbf24b3c73409a5385028d")
            .unwrap();

    let ds = MockDataSource::with_write_set(zero_sdt());
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

    let compiler = Compiler::new(MockDataSource::with_write_set(zero_sdt()));
    let compiler_service = CompilerService::new(compiler);
    let error_status = compiler_service.compile(request).await.unwrap_err();
    assert_eq!(error_status.message(), "Address is not a valid bech32");
}

#[tokio::test]
async fn test_compilation_error_on_expected_an_expression_term() {
    let source_text = r#"
            fun main() {
                let a: u128;
                return;
            }
        "#;
    let compilation_result = compile_source_file(source_text, ContractType::Script)
        .await
        .unwrap()
        .into_inner();
    assert!(compilation_result.errors[0].contains("Unused local 'a'"));
}
