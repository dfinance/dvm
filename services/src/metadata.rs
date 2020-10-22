use libra::prelude::SignatureToken;
use libra::file_format::Kind;
use lang::bytecode::metadata::{extract_bytecode_metadata, Metadata, FunctionMeta, StructMeta};
use crate::{tonic, api};
use tonic::{Request, Response, Status};
use api::grpc::metadata_grpc::dvm_bytecode_metadata_server::DvmBytecodeMetadata;
use info::metrics::meter::ScopeMeter;
use info::metrics::execution::ExecutionResult;
use dvm_net::api::grpc::metadata_grpc::{
    Metadata as GrpsMetadata, Bytecode, metadata::Meta, ScriptMeta, ModuleMeta, Function, Struct,
    Field,
};
use dvm_net::api::grpc::types::VmTypeTag;

/// Metadata service.
/// Provides a function to retrieve metadata for the script.
#[derive(Default, Clone)]
pub struct MetadataService;

#[tonic::async_trait]
impl DvmBytecodeMetadata for MetadataService {
    async fn get_metadata(
        &self,
        request: Request<Bytecode>,
    ) -> Result<Response<GrpsMetadata>, Status> {
        let mut meter = ScopeMeter::new("metadata");

        let response = extract_bytecode_metadata(&request.into_inner().code)
            .map_err(|err| Status::invalid_argument(err.to_string()))
            .and_then(map_metadata)
            .map(Response::new);

        match response {
            Ok(resp) => {
                meter.set_result(ExecutionResult::new(true, 200, 0));
                Ok(resp)
            }
            Err(status) => {
                meter.set_result(ExecutionResult::new(false, 400, 0));
                Err(status)
            }
        }
    }
}

fn map_metadata(meta: Metadata) -> Result<GrpsMetadata, Status> {
    let meta = match meta {
        Metadata::Script {
            type_parameters,
            arguments,
        } => Meta::Script(map_script_meta(type_parameters, arguments)?),
        Metadata::Module {
            name,
            functions,
            structs,
        } => Meta::Module(map_module_meta(name, functions, structs)),
    };
    Ok(GrpsMetadata { meta: Some(meta) })
}

fn map_script_meta(
    type_parameters: Vec<Kind>,
    arguments: Vec<SignatureToken>,
) -> Result<ScriptMeta, Status> {
    let type_parameters = type_parameters
        .iter()
        .map(|k| match k {
            Kind::All => "all".to_owned(),
            Kind::Resource => "resource".to_owned(),
            Kind::Copyable => "copyable".to_owned(),
        })
        .collect();

    let mut args = Vec::with_capacity(arguments.len());
    let mut signers_count = 0;

    for sign_type in arguments.iter() {
        match sign_type {
            SignatureToken::Bool => args.push(VmTypeTag::Bool as i32),
            SignatureToken::Address => args.push(VmTypeTag::Address as i32),
            SignatureToken::Vector(v_type) => {
                if v_type.as_ref() == &SignatureToken::U8 {
                    args.push(VmTypeTag::Vector as i32)
                } else {
                    return Err(Status::invalid_argument(format!(
                        "Unsupported main() signature. Unexpected vector<{:?}> type.",
                        v_type
                    )));
                }
            }
            SignatureToken::U8 => args.push(VmTypeTag::U8 as i32),
            SignatureToken::U64 => args.push(VmTypeTag::U64 as i32),
            SignatureToken::U128 => args.push(VmTypeTag::U128 as i32),
            SignatureToken::Signer => signers_count += 1,
            SignatureToken::Reference(reference) => {
                if reference.as_ref() == &SignatureToken::Signer {
                    signers_count += 1;
                } else {
                    return Err(Status::invalid_argument(
                        "Unsupported main() signature. Unexpected reference type.",
                    ));
                }
            }
            _ => {
                return Err(Status::invalid_argument("Unsupported main() signature"));
            }
        }
    }

    Ok(ScriptMeta {
        signers_count,
        type_parameters,
        arguments: args,
    })
}

fn map_module_meta(
    name: String,
    functions: Vec<FunctionMeta>,
    structs: Vec<StructMeta>,
) -> ModuleMeta {
    let functions = functions
        .into_iter()
        .map(|func| Function {
            name: func.name,
            is_public: func.is_public,
            is_native: func.is_native,
            type_parameters: func.type_params,
            arguments: func.arguments,
            returns: func.ret,
        })
        .collect();

    let types = structs
        .into_iter()
        .map(|strct| Struct {
            name: strct.name,
            is_resource: strct.is_resource,
            type_parameters: strct.type_params,
            field: strct
                .fields
                .into_iter()
                .map(|f| Field {
                    name: f.name,
                    r#type: f.f_type,
                })
                .collect(),
        })
        .collect();

    ModuleMeta {
        name,
        types,
        functions,
    }
}
