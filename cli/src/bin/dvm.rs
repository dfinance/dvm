//! Definance Virtual Machine
//! server implementation on tonic & tokio.
//! Run with `cargo run --bin dvm "http://[::1]:50051" "http://[::1]:50052"`

#[macro_use]
extern crate log;

use http::Uri;
use clap::Clap;

use tonic::transport::Server;
use futures::future::FutureExt;

use compiler::Compiler;
use services::compiler::CompilerService;
use services::metadata::MetadataService;

use dvm_net::{prelude::*, api, tonic};
use api::grpc::vm_grpc::vm_compiler_server::VmCompilerServer;
use api::grpc::vm_grpc::vm_multiple_sources_compiler_server::VmMultipleSourcesCompilerServer;
use api::grpc::vm_grpc::vm_script_metadata_server::VmScriptMetadataServer;
use dvm_net::api::grpc::vm_grpc::{
    vm_script_executor_server::VmScriptExecutorServer,
    vm_module_publisher_server::VmModulePublisherServer,
};
use data_source::{GrpcDataSource, ModuleCache, DsMeter};
use anyhow::Result;
use services::vm::VmService;
use dvm_cli::config::*;
use dvm_cli::init;
use futures::join;
use dvm_info::config::InfoServiceConfig;
use dvm_cli::info_service::create_info_service;
use dvm_net::api::grpc::vm_grpc::vm_access_vector_server::VmAccessVectorServer;

const MODULE_CACHE: usize = 1000;

/// Definance Virtual Machine
///  combined with Move compilation server
///  powered by gRPC interface on top of TCP/IPC.
/// API described in protobuf schemas: https://github.com/dfinance/dvm-proto
#[derive(Debug, Clone, Clap)]
#[clap(name = "dvm")]
#[clap(verbatim_doc_comment)]
struct Options {
    /// Address in the form of HOST_ADDRESS:PORT.
    /// The address will be listen to by DVM and compilation server.
    /// Listening localhost by default.
    /// Supports schemes: http, ipc.
    #[clap(
        name = "listen address",
        default_value = "http://[::1]:50051",
        verbatim_doc_comment
    )]
    address: Endpoint,

    #[clap(flatten)]
    info_service: InfoServiceConfig,

    /// DataSource Server internet address.
    #[clap(
    name = "Data-Source URI",
    env = DVM_DATA_SOURCE,
    default_value = "http://[::1]:50052"
    )]
    ds: Uri,

    #[clap(flatten)]
    logging: LoggingOptions,

    #[clap(flatten)]
    integrations: IntegrationsOptions,
}

fn main() -> Result<()> {
    let options = Options::parse();
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

        // shutdown server
        match serv_term_tx.send(()) {
            Ok(_) => info!("shutting down VM server"),
            Err(err) => error!("unable to send sig into the server: {:?}", err),
        }
    });

    let (info_service, hrm) = create_info_service(options.address.clone(), options.info_service);

    // data-source client
    let ds = GrpcDataSource::new(options.ds, Some(ds_term_rx))
        .expect("Unable to instantiate GrpcDataSource.");
    let ds = ModuleCache::new(DsMeter::new(ds), MODULE_CACHE);
    // vm services
    let vm_service = VmService::new(ds.clone(), hrm);
    // comp services
    let compiler_service = CompilerService::new(Compiler::new(ds));
    let metadata_service = MetadataService::default();

    // spawn the signal-router:
    tokio::spawn(sigterm);
    // block-on the server:
    let dvm = Server::builder()
        // vm service
        .add_service(VmScriptExecutorServer::new(vm_service.clone()))
        .add_service(VmModulePublisherServer::new(vm_service.clone()))
        // comp services
        .add_service(VmCompilerServer::new(compiler_service.clone()))
        .add_service(VmMultipleSourcesCompilerServer::new(compiler_service))
        .add_service(VmScriptMetadataServer::new(metadata_service.clone()))
        .add_service(VmAccessVectorServer::new(metadata_service))
        // serve
        .serve_ext_with_shutdown(options.address, serv_term_rx.map(|_| ()))
        .map(|res| {
            info!("VM server is shutted down");
            res
        });

    if let Some(info_service) = info_service {
        let (_info_service, dvm) = join!(info_service, dvm);
        dvm.expect("Dvm internal error");
    } else {
        dvm.await.expect("Dvm internal error");
    }

    Ok(())
}
