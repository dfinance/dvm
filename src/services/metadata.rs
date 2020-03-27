use dvm_api::tonic;
use tonic::{Request, Response, Status};
use libra::vm;
use vm::file_format::SignatureToken;
use lang::bytecode::extract_script_params;
use crate::compiled_protos::vm_grpc::{Signature, VmScript, VmTypeTag};
use crate::compiled_protos::vm_grpc::vm_script_metadata_server::VmScriptMetadata;

#[derive(Default)]
pub struct MetadataService {}

#[tonic::async_trait]
impl VmScriptMetadata for MetadataService {
    async fn get_signature(
        &self,
        request: Request<VmScript>,
    ) -> Result<Response<Signature>, Status> {
        let params = extract_script_params(&request.into_inner().code)
            .map_err(|err| Status::invalid_argument(err.to_string()))?;

        let mut arg_types = Vec::with_capacity(params.len());
        for sign_type in params.iter() {
            let tag = match sign_type {
                SignatureToken::Bool => VmTypeTag::Bool,
                SignatureToken::Address => VmTypeTag::Address,
                SignatureToken::ByteArray => VmTypeTag::ByteArray,
                SignatureToken::U8 => VmTypeTag::U8,
                SignatureToken::U64 => VmTypeTag::U64,
                SignatureToken::U128 => VmTypeTag::U128,
                SignatureToken::Struct(_, _) => VmTypeTag::Struct,
                _ => return Err(Status::unimplemented("Unsupported main() signature")),
            };
            arg_types.push(tag)
        }
        Ok(Response::new(Signature::new(&arg_types)))
    }
}
