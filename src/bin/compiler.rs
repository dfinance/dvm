use std::cell::RefCell;
use std::net::SocketAddr;
use std::ops::DerefMut;

use anyhow::Result;
use bytecode_verifier::VerifiedModule;
use libra_types::account_address::AccountAddress;
use structopt::StructOpt;
use tokio::time::Duration;
use tonic::{Request, Response, Status};
use tonic::transport::{Channel, Server, Uri};
use vm::CompiledModule;

use move_vm_in_cosmos::compiled_protos::ds_grpc::ds_service_client::DsServiceClient;
use move_vm_in_cosmos::compiled_protos::vm_grpc::{ContractType, MvIrSourceFile};
use move_vm_in_cosmos::compiled_protos::vm_grpc::vm_compiler_server::{VmCompiler, VmCompilerServer};
use move_vm_in_cosmos::compiler;
use move_vm_in_cosmos::compiler::mvir::{CompilerService, extract_imports, DsClient};
use move_vm_in_cosmos::test_kit::Lang;
use move_vm_in_cosmos::compiled_protos::ds_grpc::{DsAccessPath, DsRawResponse};

#[derive(Debug, StructOpt, Clone)]
struct Options {
    #[structopt(help = "Address in the form of HOST_ADDRESS:PORT")]
    address: SocketAddr,
    #[structopt(help = "DataSource Server internet address")]
    ds: Uri,
}

#[tokio::main]
async fn main() -> Result<()> {
    let address = Options::from_args().address;
    let ds_address = Options::from_args().ds;

    println!("Connecting to ds server...");
    let ds_client = loop {
        match DsServiceClient::connect(ds_address.clone()).await {
            Ok(client) => break client,
            Err(_) => tokio::time::delay_for(Duration::from_secs(1)).await,
        }
    };
    println!("Connected to ds server");

    let compiler_service = CompilerService::new(Box::new(ds_client));

    Server::builder()
        .add_service(VmCompilerServer::new(compiler_service))
        .serve(address)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use ir_to_bytecode::parser::ast::ModuleIdent;
    use libra_types::access_path::AccessPath;
    use vm::CompiledModule;
    use vm::file_format::{CompiledScript, ModuleHandleIndex};
    use vm::printers::TableAccess;

    use move_vm_in_cosmos::compiled_protos::ds_grpc::DsAccessPath;
    use move_vm_in_cosmos::compiled_protos::vm_grpc::{ContractType};
    use move_vm_in_cosmos::compiler::mvir::extract_imports;
    use move_vm_in_cosmos::move_lang::{Code, parse_program};

    use super::*;

    //    fn new_source_file(
    //        source: &str,
    //        r#type: ContractType,
    //        lang: VmLang,
    //        address: AccountAddress,
    //    ) -> MvSourceFile {
    //        SourceFile {
    //            source: source.to_string().into_bytes(),
    //            r#type: r#type as i32,
    //            lang: lang as i32,
    //            address: address.to_string().into_bytes(),
    //        }
    //    }
    //
    //    #[tokio::test]
    //    async fn test_compile_mvir_module() {
    //        let source_text = r"
    //            module M {}
    //        ";
    //        let address = AccountAddress::random();
    //        let source_file = new_source_file(source_text, ContractType::Module, VmLang::MvIr, address);
    //        let request = Request::new(source_file);
    //
    //        let compiled_module_code = GrpcCompilerService::default()
    //            .compile(request)
    //            .await
    //            .unwrap()
    //            .into_inner()
    //            .bytecode;
    //
    //        let deserialized = CompiledModule::deserialize(&compiled_module_code[..])
    //            .unwrap()
    //            .into_inner();
    //        let module_name = deserialized
    //            .get_identifier_at(
    //                deserialized
    //                    .get_module_at(ModuleHandleIndex::new(0))
    //                    .unwrap()
    //                    .name,
    //            )
    //            .unwrap()
    //            .as_str();
    //        assert_eq!(module_name, "M")
    //    }
    //
    //    #[tokio::test]
    //    async fn test_compile_mvir_script() {
    //        let source_text = r"
    //            main() {return;}
    //        ";
    //        let address = AccountAddress::random();
    //        let source_file = new_source_file(source_text, ContractType::Script, VmLang::MvIr, address);
    //        let request = Request::new(source_file);
    //
    //        let compiled_module_code = GrpcCompilerService::default()
    //            .compile(request)
    //            .await
    //            .unwrap()
    //            .into_inner()
    //            .bytecode;
    //
    //        let deserialized = CompiledScript::deserialize(&compiled_module_code[..])
    //            .unwrap()
    //            .into_inner();
    //        assert_eq!(
    //            deserialized
    //                .get_identifier_at(deserialized.function_handles[0].name)
    //                .unwrap()
    //                .to_string(),
    //            "main"
    //        );
    //    }
    //
    //    #[tokio::test]
    //    async fn test_compiler_errors() {
    //        let source_text = r"
    //            import 0x0.LibraAccount;
    //            import 0x1.LibraCore;
    //            main() {return;}
    //        ";
    //        let address = AccountAddress::random();
    //        let source_file = new_source_file(source_text, ContractType::Script, VmLang::MvIr, address);
    //        let request = Request::new(source_file);
    //
    //        let imports = extract_imports(source_text, ContractType::Script);
    //    }
}
