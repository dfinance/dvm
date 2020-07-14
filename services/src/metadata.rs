use libra::{prelude::*, vm::*};
use lang::bytecode::extract_script_params;
use crate::{tonic, api};
use tonic::{Request, Response, Status};
use api::grpc::vm_grpc::vm_script_metadata_server::VmScriptMetadata;
use api::grpc::vm_grpc::vm_access_vector_server::VmAccessVector;
use api::grpc::vm_grpc::{Signature, VmScript, VmTypeTag};
use info::metrics::meter::ScopeMeter;
use info::metrics::execution::ExecutionResult;
use dvm_net::api::grpc::vm_grpc::{AccessVector, StructIdent, LcsTag, LcsType};
use std::convert::TryFrom;
use anyhow::Error;

/// Metadata service.
/// Provides a function to retrieve metadata for the script.
#[derive(Default, Clone)]
pub struct MetadataService {}

#[tonic::async_trait]
impl VmScriptMetadata for MetadataService {
    /// Gets script signature.
    async fn get_signature(
        &self,
        request: Request<VmScript>,
    ) -> Result<Response<Signature>, Status> {
        let mut meter = ScopeMeter::new("script_metadata");
        let params = extract_script_params(&request.into_inner().code).map_err(|err| {
            meter.set_result(ExecutionResult::new(false, 400, 0));
            Status::invalid_argument(err.to_string())
        })?;

        let mut arg_types = Vec::with_capacity(params.len());
        for sign_type in params.iter() {
            let tag = match sign_type {
                SignatureToken::Bool => VmTypeTag::Bool,
                SignatureToken::Address => VmTypeTag::Address,
                SignatureToken::Vector(_) => VmTypeTag::Vector,
                SignatureToken::U8 => VmTypeTag::U8,
                SignatureToken::U64 => VmTypeTag::U64,
                SignatureToken::U128 => VmTypeTag::U128,
                SignatureToken::Reference(reference) => {
                    if reference.as_ref() == &SignatureToken::Signer {
                        // signer is not explicit parameter. Ignore it.
                        continue;
                    } else {
                        meter.set_result(ExecutionResult::new(false, 400, 0));
                        return Err(Status::unimplemented(
                            "Unsupported main() signature. Unexpected reference type.",
                        ));
                    }
                }
                _ => {
                    meter.set_result(ExecutionResult::new(false, 400, 0));
                    return Err(Status::unimplemented("Unsupported main() signature"));
                }
            };
            arg_types.push(tag)
        }

        meter.set_result(ExecutionResult::new(true, 200, 0));
        Ok(Response::new(Signature::new(&arg_types)))
    }
}

#[tonic::async_trait]
impl VmAccessVector for MetadataService {
    async fn get_access_vector(
        &self,
        request: Request<StructIdent>,
    ) -> Result<Response<AccessVector>, Status> {
        let mut meter = ScopeMeter::new("get_access_vector");
        match Ident::try_from(request.into_inner()) {
            Ok(ident) => {
                meter.set_result(ExecutionResult::new(true, 200, 0));
                Ok(Response::new(AccessVector {
                    access_vector: ident.as_ref().access_vector(),
                }))
            }
            Err(err) => {
                meter.set_result(ExecutionResult::new(false, 400, 0));
                Err(Status::unimplemented(format!(
                    "Unsupported struct signature. {}",
                    err
                )))
            }
        }
    }
}

/// StructTag wrapper.
struct Ident(StructTag);

impl AsRef<StructTag> for Ident {
    fn as_ref(&self) -> &StructTag {
        &self.0
    }
}

impl Into<StructTag> for Ident {
    fn into(self) -> StructTag {
        self.0
    }
}

impl TryFrom<StructIdent> for Ident {
    type Error = Error;

    fn try_from(value: StructIdent) -> Result<Self, Self::Error> {
        let type_params = value
            .type_params
            .into_iter()
            .map(|p| TypeIdent::try_from(p).map(|t| t.into()))
            .collect::<Result<Vec<TypeTag>, Error>>()?;

        Ok(Ident(StructTag {
            address: AccountAddress::try_from(value.address)?,
            module: Identifier::new(value.module)?,
            name: Identifier::new(value.name)?,
            type_params,
        }))
    }
}

/// TypeTag wrapper.
struct TypeIdent(TypeTag);

impl Into<TypeTag> for TypeIdent {
    fn into(self) -> TypeTag {
        self.0
    }
}

impl TryFrom<LcsTag> for TypeIdent {
    type Error = Error;

    fn try_from(value: LcsTag) -> Result<Self, Self::Error> {
        let lcs_type =
            LcsType::from_i32(value.type_tag).ok_or_else(|| anyhow!("Invalid type tag."))?;

        let tag = match lcs_type {
            LcsType::LcsBool => TypeTag::Bool,
            LcsType::LcsU64 => TypeTag::U64,
            LcsType::LcsVector => {
                let vec_type = value
                    .vector_type
                    .ok_or_else(|| anyhow!("Vector_Type is required for LcsType::LcsVector."))?;
                TypeTag::Vector(Box::new(
                    TypeIdent::try_from(vec_type.as_ref().clone())?.into(),
                ))
            }
            LcsType::LcsAddress => TypeTag::Address,
            LcsType::LcsU8 => TypeTag::U8,
            LcsType::LcsU128 => TypeTag::U128,
            LcsType::LcsSigner => TypeTag::Signer,
            LcsType::LcsStruct => {
                let struct_ident = value
                    .struct_ident
                    .ok_or_else(|| anyhow!("StructIdent is required for LcsType::LcsStruct."))?;
                TypeTag::Struct(Ident::try_from(struct_ident)?.into())
            }
        };

        Ok(TypeIdent(tag))
    }
}
