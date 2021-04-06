use dvm_net::tonic;
use tonic::{Request, Code};
use libra::account::AccountAddress;
use data_source::MockDataSource;
use dvm_services::metadata::MetadataService;
use compiler::Compiler;
use dvm_net::api::grpc::VmTypeTag;
use dvm_net::api::grpc::Bytecode;
use dvm_net::api::grpc::dvm_bytecode_metadata_server::DvmBytecodeMetadata;
use dvm_net::api::grpc::metadata::Meta;

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
    let request = Request::new(Bytecode {
        code: script_bytecode,
    });
    let bytecode_meta = metadata_service
        .get_metadata(request)
        .await
        .unwrap()
        .into_inner();

    if let Meta::Script(script_meta) = bytecode_meta.meta.unwrap() {
        assert_eq!(script_meta.arguments.len(), 0);
        assert_eq!(script_meta.signers_count, 0);
        assert_eq!(script_meta.type_parameters.len(), 0);
    } else {
        panic!("Expected script metadata");
    }
}

#[tokio::test]
async fn test_no_arguments_for_script_with_signer() {
    let source_text = r"
            script {
            fun main<T, F: resource, D: copyable>(_account: &signer) {
            }
            }
        ";
    let compiler = Compiler::new(MockDataSource::new());
    let script_bytecode = compiler
        .compile(source_text, Some(AccountAddress::random()))
        .unwrap();
    let metadata_service = MetadataService::default();
    let request = Request::new(Bytecode {
        code: script_bytecode,
    });
    let bytecode_meta = metadata_service
        .get_metadata(request)
        .await
        .unwrap()
        .into_inner();

    if let Meta::Script(script_meta) = bytecode_meta.meta.unwrap() {
        assert_eq!(script_meta.arguments.len(), 0);
        assert_eq!(script_meta.signers_count, 1);
        assert_eq!(
            script_meta.type_parameters,
            vec![
                "all".to_owned(),
                "resource".to_owned(),
                "copyable".to_owned()
            ]
        );
    } else {
        panic!("Expected script metadata");
    }
}

#[tokio::test]
async fn test_multiple_arguments_for_move_script() {
    let source_text = r"
            script {
            fun main<T, T2>(_account: &signer, _account_1: &signer, _recipient: address, _amount: u128, _denom: vector<u8>) {
            }
            }
        ";
    let compiler = Compiler::new(MockDataSource::new());
    let script_bytecode = compiler
        .compile(source_text, Some(AccountAddress::random()))
        .unwrap();
    let metadata_service = MetadataService::default();
    let request = Request::new(Bytecode {
        code: script_bytecode,
    });

    let bytecode_meta = metadata_service
        .get_metadata(request)
        .await
        .unwrap()
        .into_inner();
    if let Meta::Script(script_meta) = bytecode_meta.meta.unwrap() {
        assert_eq!(
            script_meta.arguments,
            vec![
                VmTypeTag::Address as i32,
                VmTypeTag::U128 as i32,
                VmTypeTag::Vector as i32
            ]
        );
        assert_eq!(script_meta.signers_count, 2);
        assert_eq!(script_meta.type_parameters.len(), 2);
    } else {
        panic!("Expected script metadata");
    }
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
    let request = Request::new(Bytecode {
        code: script_bytecode,
    });
    let err_status = metadata_service.get_metadata(request).await.unwrap_err();
    assert_eq!(err_status.code(), Code::InvalidArgument);
    assert_eq!(
        err_status.message(),
        "status UNKNOWN_SERIALIZED_TYPE of type Deserialization"
    );
}

#[tokio::test]
async fn test_unsupported_script_arguments() {
    let source_text = r"
            script {
            fun main(_denom: vector<u64>) {
            }
            }
        ";
    let compiler = Compiler::new(MockDataSource::new());
    let script_bytecode = compiler
        .compile(source_text, Some(AccountAddress::random()))
        .unwrap();
    let metadata_service = MetadataService::default();
    let request = Request::new(Bytecode {
        code: script_bytecode,
    });
    let err_status = metadata_service.get_metadata(request).await.unwrap_err();
    assert_eq!(err_status.code(), Code::InvalidArgument);
    assert_eq!(
        err_status.message(),
        "Unsupported main() signature. Unexpected vector<U64> type."
    );
}
