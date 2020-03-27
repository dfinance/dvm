use libra::libra_types;
use libra_types::account_address::AccountAddress;
use dvm_api::tonic;
use tonic::{Request, Code};

use dvm::compiled_protos::vm_grpc::{VmScript, VmTypeTag};
use dvm::compiled_protos::vm_grpc::vm_script_metadata_server::VmScriptMetadata;
use dvm::services::metadata::MetadataService;
use lang::compiler::Compiler;
use data_source::MockDataSource;

#[tokio::test]
async fn test_no_arguments_for_mvir_script() {
    let source_text = r"
            main() {
                return;
            }
        ";
    let compiler = Compiler::new(MockDataSource::new());
    let script_bytecode = compiler
        .compile(source_text, &AccountAddress::random())
        .unwrap();
    let metadata_service = MetadataService::default();
    let request = Request::new(VmScript::new(script_bytecode));
    let arguments = metadata_service
        .get_signature(request)
        .await
        .unwrap()
        .into_inner()
        .arguments;
    assert!(arguments.is_empty());
}

#[tokio::test]
async fn test_multiple_arguments_for_mvir_script() {
    let source_text = r"
            main(recipient: address, amount: u128, denom: bytearray) {
                return;
            }
        ";
    let compiler = Compiler::new(MockDataSource::new());
    let script_bytecode = compiler
        .compile(source_text, &AccountAddress::random())
        .unwrap();
    let metadata_service = MetadataService::default();
    let request = Request::new(VmScript::new(script_bytecode));
    let arguments = metadata_service
        .get_signature(request)
        .await
        .unwrap()
        .into_inner()
        .arguments;
    assert_eq!(
        arguments,
        vec![
            VmTypeTag::Address as i32,
            VmTypeTag::U128 as i32,
            VmTypeTag::ByteArray as i32
        ]
    );
}

#[tokio::test]
async fn test_cannot_deserialize_bytecode() {
    let source_text = r"
            main(recipient: address, amount: u128, denom: bytearray) {
                return;
            }
        ";
    let compiler = Compiler::new(MockDataSource::new());
    let mut script_bytecode = compiler
        .compile(source_text, &AccountAddress::random())
        .unwrap();
    script_bytecode[13] = 0xff;
    let metadata_service = MetadataService::default();
    let request = Request::new(VmScript::new(script_bytecode));
    let err_status = metadata_service.get_signature(request).await.unwrap_err();
    assert_eq!(err_status.code(), Code::InvalidArgument);
    assert_eq!(
        err_status.message(),
        "Cannot deserialize script from provided bytecode. Error:[status BAD_HEADER_TABLE of type Deserialization]"
    );
}
