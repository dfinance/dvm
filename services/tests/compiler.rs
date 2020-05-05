use libra::libra_vm;
use libra_vm::access::ScriptAccess;
use libra_vm::CompiledModule;
use libra_vm::file_format::{Bytecode, ModuleHandleIndex, FunctionHandleIndex};
use libra_vm::file_format::CompiledScript;
use libra::libra_types::account_address::AccountAddress;
use dvm_api::tonic;
use tonic::{Request, Response, Status};

use lang::{
    compiler::{Compiler, preprocessor::str_xxhash},
    stdlib::build_std,
};
use data_source::MockDataSource;
use dvm_api::grpc::vm_grpc::{CompilationResult, ContractType, MvIrSourceFile};
use dvm_services::compiler::CompilerService;
use dvm_api::grpc::vm_grpc::vm_compiler_server::VmCompiler;

fn new_source_file(source: &str, r#type: ContractType, address: &AccountAddress) -> MvIrSourceFile {
    MvIrSourceFile {
        text: source.to_string(),
        r#type: r#type as i32,
        address: address.to_vec(),
    }
}

fn new_source_file_request(source_text: &str, r#type: ContractType) -> Request<MvIrSourceFile> {
    let address = AccountAddress::random();
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
async fn test_compile_module() {
    let source_text = r"
            module M {
                public fun method() {
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
async fn test_compile_script() {
    let source_text = r"
            fun main() {
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
    assert_eq!(compiled_script.code().code, vec![Bytecode::Ret]);
}

#[tokio::test]
async fn test_compile_script_with_dependencies() {
    let source_text = "
            use 0x0::Oracle;
            fun main() {
                Oracle::get_price(#\"USDBTC\");
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
    assert_eq!(
        compiled_script.code().code,
        vec![
            Bytecode::LdU64(str_xxhash("usdbtc")),
            Bytecode::Call(FunctionHandleIndex(0)),
            Bytecode::Pop,
            Bytecode::Ret
        ]
    );

    let imported_module_handle = compiled_script.module_handle_at(ModuleHandleIndex::new(1u16));
    assert_eq!(
        compiled_script
            .identifier_at(imported_module_handle.name)
            .to_string(),
        "Oracle"
    );
}

#[tokio::test]
async fn test_required_libracoin_dependency_is_not_available() {
    let source_text = r"
            use 0x0::Coin;
            fun main() {
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
        r#"Module with path [ModuleId { address: 000000000000000000000000000000000000000000000000, name: Identifier("Coin") }] not found"#
    )
}

#[tokio::test]
async fn test_allows_for_bech32_addresses() {
    let source_text = r"
            use wallet1me0cdn52672y7feddy7tgcj6j4dkzq2su745vh::Hash;
            fun main() {
                Hash::hash();
            }
        ";

    let source_file_request = new_source_file_request(source_text, ContractType::Script);

    let libra_address =
        AccountAddress::from_hex_literal("0xde5f86ce8ad7944f272d693cb4625a955b61015000000000")
            .unwrap();

    let ds = MockDataSource::with_write_set(build_std());
    let compiler = Compiler::new(ds.clone());
    let hash = compiler
        .compile(
            "\
        module Hash {
            public fun hash(){}
        }
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
