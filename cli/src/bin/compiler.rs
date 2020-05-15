//! Compilation server implementation on tonic & tokio.
//! Run with `cargo run --bin compiler "http://[::1]:50053" "http://[::1]:50052"`

#[macro_use]
extern crate log;

use anyhow::Result;
use structopt::StructOpt;

use dvm_net::tonic;
use tonic::transport::{Server, Uri};
use futures::future::{lazy, FutureExt};

use data_source::{GrpcDataSource, ModuleCache};

use compiler::Compiler;
use services::compiler::CompilerService;
use services::metadata::MetadataService;
use dvm_net::prelude::*;
use dvm_net::api;
use api::grpc::vm_grpc::vm_compiler_server::VmCompilerServer;
use api::grpc::vm_grpc::vm_multiple_sources_compiler_server::VmMultipleSourcesCompilerServer;
use api::grpc::vm_grpc::vm_script_metadata_server::VmScriptMetadataServer;
use dvm_cli::config::{LoggingOptions, IntegrationsOptions, DVM_DATA_SOURCE};
use dvm_cli::{init, logging, ShutdownSignal};

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
    let (sigterm, _guard) = init(&options.logging, &options.integrations);
    main_internal(options, sigterm)
}

#[tokio::main]
async fn main_internal(options: Options, sigterm: ShutdownSignal) -> Result<()> {
    let ds = GrpcDataSource::new(options.ds).expect("Unable to instantiate GrpcDataSource.");
    let ds = ModuleCache::new(ds, MODULE_CACHE);
    let compiler_service = CompilerService::new(Compiler::new(ds));
    let metadata_service = MetadataService::default();
    let sigterm = lazy(move |_| sigterm.recv()).map(|_| logging::log_shutdown(Some("compiler")));

    info!("Compilation server listening on {}", options.address);
    Server::builder()
        .add_service(VmCompilerServer::new(compiler_service.clone()))
        .add_service(VmMultipleSourcesCompilerServer::new(compiler_service))
        .add_service(VmScriptMetadataServer::new(metadata_service))
        .serve_ext_with_shutdown(options.address, sigterm)
        .await
        .expect("internal fail");
    Ok(())
}
