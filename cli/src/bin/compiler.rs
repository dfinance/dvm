//! Compilation server implementation on tonic & tokio.
//! Run with `cargo run --bin compiler "http://[::1]:50053" "http://[::1]:50052"`

#[macro_use]
extern crate log;

use anyhow::Result;
use structopt::StructOpt;

use dvm_net::tonic;
use tonic::transport::{Server, Uri};
use futures::future::FutureExt;

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
use dvm_cli::init;

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
    let _guard = init(&options.logging, &options.integrations);
    main_internal(options)
}

#[tokio::main]
async fn main_internal(options: Options) -> Result<()> {
    let (serv_term_tx, serv_term_rx) = futures::channel::oneshot::channel();
    let (ds_term_tx, ds_term_rx) = tokio::sync::oneshot::channel();
    let sigterm = dvm_cli::init_sigterm_handler_fut(move || {
        // shutdown DS
        match ds_term_tx.send(()) {
            Ok(_) => info!("shutting down DS client"),
            Err(err) => error!("unable to send sig into the DS client: {:?}", err),
        }

        // shutdown compilation server
        match serv_term_tx.send(()) {
            Ok(_) => info!("shutting down compilation server"),
            Err(err) => error!("unable to send sig into the server: {:?}", err),
        }
    });

    let ds = GrpcDataSource::new(options.ds, Some(ds_term_rx))
        .expect("Unable to instantiate GrpcDataSource.");
    let ds = ModuleCache::new(ds, MODULE_CACHE);
    let compiler_service = CompilerService::new(Compiler::new(ds));
    let metadata_service = MetadataService::default();

    tokio::select! {
        _ = sigterm => {},
        res = Server::builder().add_service(VmCompilerServer::new(compiler_service.clone()))
                               .add_service(VmMultipleSourcesCompilerServer::new(compiler_service))
                               .add_service(VmScriptMetadataServer::new(metadata_service))
                               .serve_ext_with_shutdown(options.address, serv_term_rx.map(|_| ())) => {
            info!("Compilation server is shutted down");
            res.expect("internal fail");
        }
    }

    Ok(())
}
