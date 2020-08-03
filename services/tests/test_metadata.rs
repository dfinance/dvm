use libra::{prelude::*, vm::*};
use dvm_net::{tonic, api};
use tonic::{Request, Code};

use data_source::MockDataSource;
use dvm_services::metadata::MetadataService;
use api::grpc::vm_grpc::{VmScript, VmTypeTag};
use api::grpc::vm_grpc::vm_script_metadata_server::VmScriptMetadata;
use compiler::Compiler;
use api::grpc::vm_grpc::vm_access_vector_server::VmAccessVector;
use api::grpc::vm_grpc::StructIdent;
use dvm_net::api::grpc::vm_grpc::{LcsTag, LcsType};

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
        "Cannot deserialize script from provided bytecode. Error:[PartialVMError with status UNKNOWN_SERIALIZED_TYPE]"
    );
}

#[tokio::test]
async fn test_access_vector() {
    let metadata_service = MetadataService::default();
    let address = AccountAddress::random();

    let struct_tag = StructTag {
        address,
        module: Identifier::new("ModuleName".to_owned()).unwrap(),
        name: Identifier::new("StructName".to_owned()).unwrap(),
        type_params: vec![
            TypeTag::Bool,
            TypeTag::U8,
            TypeTag::U64,
            TypeTag::Address,
            TypeTag::Signer,
            TypeTag::Struct(StructTag {
                address: CORE_CODE_ADDRESS,
                module: Identifier::new("InnerModule".to_owned()).unwrap(),
                name: Identifier::new("InnerStruct".to_owned()).unwrap(),
                type_params: vec![TypeTag::Vector(Box::new(TypeTag::Address))],
            }),
            TypeTag::Vector(Box::new(TypeTag::Vector(Box::new(TypeTag::U8)))),
        ],
    };

    let ident = StructIdent {
        address: address.to_vec(),
        module: "ModuleName".to_string(),
        name: "StructName".to_string(),
        type_params: vec![
            LcsTag {
                type_tag: LcsType::LcsBool as i32,
                vector_type: None,
                struct_ident: None,
            },
            LcsTag {
                type_tag: LcsType::LcsU8 as i32,
                vector_type: None,
                struct_ident: None,
            },
            LcsTag {
                type_tag: LcsType::LcsU64 as i32,
                vector_type: None,
                struct_ident: None,
            },
            LcsTag {
                type_tag: LcsType::LcsAddress as i32,
                vector_type: None,
                struct_ident: None,
            },
            LcsTag {
                type_tag: LcsType::LcsSigner as i32,
                vector_type: None,
                struct_ident: None,
            },
            LcsTag {
                type_tag: LcsType::LcsStruct as i32,
                vector_type: None,
                struct_ident: Some(StructIdent {
                    address: CORE_CODE_ADDRESS.to_vec(),
                    module: "InnerModule".to_string(),
                    name: "InnerStruct".to_string(),
                    type_params: vec![LcsTag {
                        type_tag: LcsType::LcsVector as i32,
                        vector_type: Some(Box::new(LcsTag {
                            type_tag: LcsType::LcsAddress as i32,
                            vector_type: None,
                            struct_ident: None,
                        })),
                        struct_ident: None,
                    }],
                }),
            },
            LcsTag {
                type_tag: LcsType::LcsVector as i32,
                vector_type: Some(Box::new(LcsTag {
                    type_tag: LcsType::LcsVector as i32,
                    vector_type: Some(Box::new(LcsTag {
                        type_tag: LcsType::LcsU8 as i32,
                        vector_type: None,
                        struct_ident: None,
                    })),
                    struct_ident: None,
                })),
                struct_ident: None,
            },
        ],
    };

    let request = Request::new(ident);
    let access_vector = metadata_service
        .get_access_vector(request)
        .await
        .unwrap()
        .into_inner()
        .access_vector;

    assert_eq!(access_vector, struct_tag.access_vector());
}
