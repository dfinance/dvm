use libra_types::account_address::AccountAddress;
use tonic::Request;

use move_vm_in_cosmos::compiled_protos::vm_grpc::{VmScript, VmTypeTag};
use move_vm_in_cosmos::compiled_protos::vm_grpc::vm_script_metadata_server::VmScriptMetadata;
use move_vm_in_cosmos::compiler::mvir::compile_mvir;
use move_vm_in_cosmos::vm::metadata::MetadataService;

#[tokio::test]
async fn test_no_arguments_for_mvir_script() {
    let source_text = r"
            main() {
                return;
            }
        ";
    let script_bytecode =
        compile_mvir(source_text, AccountAddress::random(), false, vec![]).unwrap();

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
    let script_bytecode =
        compile_mvir(source_text, AccountAddress::random(), false, vec![]).unwrap();
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
