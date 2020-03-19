use dvm_api::tonic;
use tonic::{Request, Response, Status};
use libra::vm;
use vm::file_format::{CompiledScript, SignatureToken};
use vm::printers::TableAccess;

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
        let compiled_script = CompiledScript::deserialize(&request.into_inner().code)
            .map_err(|_| {
                Status::invalid_argument("Cannot deserialize script from provided bytecode")
            })?
            .into_inner();
        let main_function = compiled_script
            .get_function_at(compiled_script.main.function)
            .unwrap();
        let main_function_signature = compiled_script
            .get_function_signature_at(main_function.signature)
            .unwrap();
        let mut arg_types = Vec::with_capacity(main_function_signature.arg_types.len());
        for sign_type in main_function_signature.arg_types.iter() {
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
