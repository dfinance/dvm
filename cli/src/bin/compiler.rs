//! Compilation server implementation on tonic & tokio.
//! Run with `cargo run --bin compiler "http://[::1]:50053" "http://[::1]:50052"`

#[macro_use]
extern crate log;

use anyhow::Result;
use structopt::StructOpt;

use dvm_api::tonic;
// use dvm_api::endpoint::Endpoint;
use tonic::transport::{Server, Uri};

use data_source::{GrpcDataSource, ModuleCache};

use lang::compiler::Compiler;
use services::compiler::CompilerService;
use services::metadata::MetadataService;
use dvm_net::prelude::*;
use dvm_api::grpc::vm_grpc::vm_compiler_server::VmCompilerServer;
use dvm_api::grpc::vm_grpc::vm_multiple_sources_compiler_server::VmMultipleSourcesCompilerServer;
use dvm_api::grpc::vm_grpc::vm_script_metadata_server::VmScriptMetadataServer;
use dvm_cli::config::{LoggingOptions, IntegrationsOptions, DVM_DATA_SOURCE};
use dvm_cli::logging;

const MODULE_CACHE: usize = 1000;

/// Move compiler with grpc interface.
///
/// API described in protobuf schemas: https://github.com/dfinance/dvm-proto
#[derive(Debug, StructOpt, Clone)]
struct Options {
    /// Address in the form of HOST_ADDRESS:PORT.
    /// The address will be listen to by this compilation server.
    /// Listening localhost by default.
    /// Supports schemes: http, ipc.
    #[structopt(
        name = "listen address",
        default_value = "http://[::1]:50053",
        verbatim_doc_comment
    )]
    address: Endpoint,

    // address: Endpoint,
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
    let _guard = logging::init(&options.logging, &options.integrations);
    main_internal(options)
}

#[tokio::main]
async fn main_internal(options: Options) -> Result<()> {
    let ds = GrpcDataSource::new(options.ds).expect("Unable to instantiate GrpcDataSource.");
    let ds = ModuleCache::new(ds, MODULE_CACHE);
    let compiler_service = CompilerService::new(Compiler::new(ds));
    let metadata_service = MetadataService::default();

    info!("Compilation server listening on {}", options.address);
    Server::builder()
        .add_service(VmCompilerServer::new(compiler_service.clone()))
        .add_service(VmMultipleSourcesCompilerServer::new(compiler_service))
        .add_service(VmScriptMetadataServer::new(metadata_service))
        .serve_with(options.address)
        .await
        .expect("internal fail");
    Ok(())
}
