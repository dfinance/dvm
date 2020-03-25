#[macro_use]
extern crate log;

use std::net::SocketAddr;
use anyhow::Result;
use structopt::StructOpt;

use dvm_api::tonic;
use tonic::transport::{Server, Uri};

use dvm::cli::config::*;
use dvm::compiled_protos::vm_grpc::vm_compiler_server::VmCompilerServer;
use dvm::compiled_protos::vm_grpc::vm_script_metadata_server::VmScriptMetadataServer;
use data_source::{GrpcDataSource, ModuleCache};

use lang::compiler::Compiler;
use dvm::services::compiler::CompilerService;
use dvm::services::metadata::MetadataService;

const MODULE_CACHE: usize = 1000;

/// Move & Mvir compiler with grpc interface.
///
/// API described in protobuf schemas: https://github.com/dfinance/dvm-proto
#[derive(Debug, StructOpt, Clone)]
struct Options {
    /// Address in the form of HOST_ADDRESS:PORT.
    /// The address will be listen to by this compilation server.
    /// Listening localhost by default.
    #[structopt(
        name = "listen address",
        default_value = "[::1]:50053",
        verbatim_doc_comment
    )]
    address: SocketAddr,

    /// DataSource Server internet address.
    #[structopt(
        name = "Data-Source URI",
        env = DVM_DATA_SOURCE,
        default_value = "http://[::1]:50052"
    )]
    ds: Uri,

    #[structopt(flatten)]
    logging: LoggingOptions,

    #[structopt(flatten)]
    integrations: IntegrationsOptions,
}

fn main() -> Result<()> {
    let options = Options::from_args();
    let _guard = dvm::cli::init(&options.logging, &options.integrations);
    main_internal(options)
}

#[tokio::main]
async fn main_internal(options: Options) -> Result<()> {
    let ds = GrpcDataSource::new(options.ds).expect("GrpcDataSource expect.");
    let ds = ModuleCache::new(ds, MODULE_CACHE);
    let compiler_service = CompilerService::new(Compiler::new(ds));
    let metadata_service = MetadataService::default();

    info!("DVM server listening on {}", options.address);
    Server::builder()
        .add_service(VmCompilerServer::new(compiler_service))
        .add_service(VmScriptMetadataServer::new(metadata_service))
        .serve(options.address)
        .await?;
    Ok(())
}
