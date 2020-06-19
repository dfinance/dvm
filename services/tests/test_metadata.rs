use libra::libra_types;
use libra_types::account_address::AccountAddress;
use dvm_net::{tonic, api};
use tonic::{Request, Code};

use data_source::MockDataSource;
use dvm_services::metadata::MetadataService;
use api::grpc::vm_grpc::{VmScript, VmTypeTag};
use api::grpc::vm_grpc::vm_script_metadata_server::VmScriptMetadata;
use compiler::Compiler;

#[tokio::test]
async fn test_no_arguments_for_script() {
    let source_text = r"
            script {
            fun main() {
            }
            }
        ";
    let compiler = Compiler::new(MockDataSource::new());
    let script_bytecode = compiler
        .compile(source_text, Some(AccountAddress::random()))
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
async fn test_no_arguments_for_script_with_signer() {
    let source_text = r"
            script {
            fun main(_account: &signer) {
            }
            }
        ";
    let compiler = Compiler::new(MockDataSource::new());
    let script_bytecode = compiler
        .compile(source_text, Some(AccountAddress::random()))
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
async fn test_multiple_arguments_for_move_script() {
    let source_text = r"
            script {
            fun main(_account: &signer, _recipient: address, _amount: u128, _denom: vector<u8>) {
            }
            }
        ";
    let compiler = Compiler::new(MockDataSource::new());
    let script_bytecode = compiler
        .compile(source_text, Some(AccountAddress::random()))
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
            VmTypeTag::Vector as i32
        ]
    );
}

#[tokio::test]
async fn test_cannot_deserialize_bytecode() {
    let source_text = r"
            script {
            fun main(_recipient: address, _amount: u128, _denom: vector<u8>) {
            }
            }
        ";
    let compiler = Compiler::new(MockDataSource::new());
    let mut script_bytecode = compiler
        .compile(source_text, Some(AccountAddress::random()))
        .unwrap();
    script_bytecode[13] = 0xff;
    let metadata_service = MetadataService::default();
    let request = Request::new(VmScript::new(script_bytecode));
    let err_status = metadata_service.get_signature(request).await.unwrap_err();
    assert_eq!(err_status.code(), Code::InvalidArgument);
    assert_eq!(
        err_status.message(),
        "Cannot deserialize script from provided bytecode. Error:[status UNKNOWN_SERIALIZED_TYPE of type Deserialization]"
    );
}
