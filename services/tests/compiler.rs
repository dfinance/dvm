use libra::{prelude::*, file_format::*};
use dvm_net::tonic;
use tonic::Request;
use lang::{stdlib::build_std};
use compiler::Compiler;
use data_source::MockDataSource;
use dvm_services::compiler::CompilerService;
use dvm_net::api::grpc::compiler_grpc::{SourceFiles, CompilationUnit, CompiledUnit as Unit};

fn new_source_file(source: &str, address: &AccountAddress) -> SourceFiles {
    SourceFiles {
        units: vec![CompilationUnit {
            text: source.to_string(),
            name: "src".to_string(),
        }],
        address: address.to_vec(),
    }
}

fn new_source_file_request(source_text: &str) -> Request<SourceFiles> {
    let address = AccountAddress::random();
    let source_file = new_source_file(source_text, &address);
    Request::new(source_file)
}

async fn compile_source_file(source_text: &str) -> Result<Vec<Unit>, String> {
    let source_file_request = new_source_file_request(source_text);

    let compiler = Compiler::new(MockDataSource::with_write_set(build_std()));
    let compiler_service = CompilerService::new(compiler);
    compiler_service.compile(source_file_request).await.unwrap()
}

#[tokio::test]
async fn test_compile_module() {
    let source_text = r"
            module M {
                public fun method() {
                }
            }
        ";
    let compilation_result = compile_source_file(source_text).await.unwrap().remove(0);
    CompiledModule::deserialize(&compilation_result.bytecode).unwrap();
}

#[tokio::test]
async fn test_compile_script() {
    let source_text = r"
            script {
            fun main() {
            }
            }
        ";
    let compilation_result = compile_source_file(source_text).await.unwrap().remove(0);
    let compiled_script = CompiledScript::deserialize(&compilation_result.bytecode[..]).unwrap();
    assert_eq!(compiled_script.code().code, vec![Bytecode::Ret]);
}

#[tokio::test]
async fn test_compile_script_with_dependencies() {
    let source_text = "
            script {
            use 0x1::Time;
            fun main() {
                Time::now();
            }
            }
        ";
    let source_file_request = new_source_file_request(source_text);

    let compiler = Compiler::new(MockDataSource::with_write_set(build_std()));

    let compiler_service = CompilerService::new(compiler);
    let compilation_result = compiler_service
        .compile(source_file_request)
        .await
        .unwrap()
        .unwrap()
        .remove(0);

    let compiled_script = CompiledScript::deserialize(&compilation_result.bytecode).unwrap();
    assert_eq!(
        compiled_script.code().code,
        vec![
            Bytecode::Call(FunctionHandleIndex(0)),
            Bytecode::Pop,
            Bytecode::Ret
        ]
    );

    let imported_module_handle = &compiled_script.module_handle_at(ModuleHandleIndex::new(0u16));
    assert_eq!(
        compiled_script
            .identifier_at(imported_module_handle.name)
            .to_string(),
        "Time"
    );
}

#[tokio::test]
async fn test_required_libracoin_dependency_is_not_available() {
    let source_text = r"
            script {
            use 0x1::Coin;
            fun main() {
            }
            }
        ";

    let source_file_request = new_source_file_request(source_text);

    let compiler = Compiler::new(MockDataSource::with_write_set(build_std()));
    let compiler_service = CompilerService::new(compiler);
    let error = compiler_service
        .compile(source_file_request)
        .await
        .unwrap()
        .unwrap_err();
    assert_eq!(
        error,
        r#"Module '0x0000000000000000000000000000000000000001::Coin' not found"#
    )
}

#[tokio::test]
async fn test_allows_for_bech32_addresses() {
    let source_text = r"
            script {
            use wallet1me0cdn52672y7feddy7tgcj6j4dkzq2su745vh::Hash;
            fun main() {
                Hash::hash();
            }
            }
        ";

    let source_file_request = new_source_file_request(source_text);

    let libra_address =
        AccountAddress::from_hex_literal("0xde5f86ce8ad7944f272d693cb4625a955b610150").unwrap();

    let ds = MockDataSource::with_write_set(build_std());
    let compiler = Compiler::new(ds.clone());
    let hash = compiler
        .compile(
            "\
        module Hash {
            public fun hash(){}
        }
        ",
            Some(libra_address),
        )
        .unwrap();
    ds.publish_module(hash).unwrap();

    let compiler_service = CompilerService::new(compiler);
    compiler_service
        .compile(source_file_request)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn test_compilation_error_on_expected_an_expression_term() {
    let source_text = r#"
            script {
            fun main() {
                let a: u128;
                return;
            }
            }
        "#;
    let error = compile_source_file(source_text).await.unwrap_err();
    assert!(error.contains("Unused local 'a'"));
}
