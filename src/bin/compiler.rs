use std::net::SocketAddr;

use anyhow::Result;
use libra_types::account_address::AccountAddress;
use structopt::StructOpt;
use tonic::{Request, Response, Status};
use tonic::transport::Server;

use move_vm_in_cosmos::compiled_protos::vm_grpc::{CompiledFile, SourceFile, VmLang};
use move_vm_in_cosmos::compiled_protos::vm_grpc::vm_compiler_server::{VmCompiler, VmCompilerServer};
use move_vm_in_cosmos::test_kit::Lang;

fn lang_from_vmlang(vm_lang: i32) -> Lang {
    if vm_lang == VmLang::MvIr as i32 {
        Lang::MvIr
    } else if vm_lang == VmLang::Move as i32 {
        Lang::Move
    } else {
        unimplemented!("Invalid VmLang i32")
    }
}

#[derive(Default)]
pub struct GrpcCompilerService {}

#[tonic::async_trait]
impl VmCompiler for GrpcCompilerService {
    async fn compile(
        &self,
        request: Request<SourceFile>,
    ) -> Result<Response<CompiledFile>, Status> {
        let source_file_data = request.into_inner();
        let compiler = lang_from_vmlang(source_file_data.lang).compiler();

        let source = match String::from_utf8(source_file_data.source) {
            Ok(s) => s,
            Err(_) => return Err(Status::invalid_argument("Source is not a valid utf8")),
        };
        let address_lit = match String::from_utf8(source_file_data.address.to_vec()) {
            Ok(address) => address,
            Err(_) => return Err(Status::invalid_argument("Address is not a valid utf8")),
        };
        let account_address = AccountAddress::from_hex_literal(&address_lit).unwrap();

        let compiled_bytecode = match source_file_data.r#type {
            0 => compiler.build_module(&source, &account_address),
            1 => compiler.build_script(&source, &account_address),
            _ => unimplemented!(),
        };
        Ok(Response::new(CompiledFile {
            bytecode: compiled_bytecode,
        }))
    }
}

#[derive(Debug, StructOpt, Clone)]
struct Options {
    #[structopt(help = "Address in the form of HOST_ADDRESS:PORT")]
    address: SocketAddr,
}

#[tokio::main]
async fn main() -> Result<()> {
    let address = Options::from_args().address;
    Server::builder()
        .add_service(VmCompilerServer::new(GrpcCompilerService::default()))
        .serve(address)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use vm::CompiledModule;
    use vm::file_format::{CompiledScript, ModuleHandleIndex};
    use vm::printers::TableAccess;

    use move_vm_in_cosmos::compiled_protos::vm_grpc::{ContractType, VmLang};

    use super::*;

    fn new_source_file(
        source: &str,
        r#type: ContractType,
        lang: VmLang,
        address: AccountAddress,
    ) -> SourceFile {
        SourceFile {
            source: source.to_string().into_bytes(),
            r#type: r#type as i32,
            lang: lang as i32,
            address: address.to_string().into_bytes(),
        }
    }

    #[tokio::test]
    async fn test_compile_mvir_module() {
        let source_text = r"
            module M {}
        ";
        let address = AccountAddress::random();
        let source_file = new_source_file(source_text, ContractType::Module, VmLang::MvIr, address);
        let request = Request::new(source_file);

        let compiled_module_code = GrpcCompilerService::default()
            .compile(request)
            .await
            .unwrap()
            .into_inner()
            .bytecode;

        let deserialized = CompiledModule::deserialize(&compiled_module_code[..])
            .unwrap()
            .into_inner();
        let module_name = deserialized
            .get_identifier_at(
                deserialized
                    .get_module_at(ModuleHandleIndex::new(0))
                    .unwrap()
                    .name,
            )
            .unwrap()
            .as_str();
        assert_eq!(module_name, "M")
    }

    #[tokio::test]
    async fn test_compile_mvir_script() {
        let source_text = r"
            main() {return;}
        ";
        let address = AccountAddress::random();
        let source_file = new_source_file(source_text, ContractType::Script, VmLang::MvIr, address);
        let request = Request::new(source_file);

        let compiled_module_code = GrpcCompilerService::default()
            .compile(request)
            .await
            .unwrap()
            .into_inner()
            .bytecode;

        let deserialized = CompiledScript::deserialize(&compiled_module_code[..])
            .unwrap()
            .into_inner();
        assert_eq!(
            deserialized
                .get_identifier_at(deserialized.function_handles[0].name)
                .unwrap()
                .to_string(),
            "main"
        );
    }
}
