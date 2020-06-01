//! Definance Virtual Machine
//! server implementation on tonic & tokio.
//! Run with `cargo run --bin dvm "http://[::1]:50051" "http://[::1]:50052"`

#[macro_use]
extern crate log;

use http::Uri;
use structopt::StructOpt;

use tonic::transport::Server;
use futures::future::FutureExt;

use dvm_net::prelude::*;
use dvm_net::tonic;
use dvm_net::api::grpc::vm_grpc::vm_service_server::VmServiceServer;
use data_source::{GrpcDataSource, ModuleCache};
use anyhow::Result;
use services::vm::VmService;
use dvm_cli::config::*;
use dvm_cli::init;

const MODULE_CACHE: usize = 1000;

/// Definance Virtual Machine with gRPC interface on top of TCP/IPC.
/// API described in protobuf schemas: https://github.com/dfinance/dvm-proto
#[derive(Debug, StructOpt, Clone)]
#[structopt(name = "dvm")]
#[structopt(verbatim_doc_comment)]
struct Options {
    /// Address in the form of HOST_ADDRESS:PORT.
    /// The address will be listen to by DVM (this) server.
    /// Listening localhost by default.
    /// Supports schemes: http, ipc.
    #[structopt(
        name = "listen address",
        default_value = "http://[::1]:50051",
        verbatim_doc_comment
    )]
    address: Endpoint,

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

        // shutdown server
        match serv_term_tx.send(()) {
            Ok(_) => info!("shutting down VM server"),
            Err(err) => error!("unable to send sig into the server: {:?}", err),
        }
    });

    let ds = GrpcDataSource::new(options.ds, Some(ds_term_rx))
        .expect("Unable to instantiate GrpcDataSource.");
    let ds = ModuleCache::new(ds, MODULE_CACHE);
    let service = VmService::new(ds);

    // spawn the signal-router:
    tokio::spawn(sigterm);
    // block-on the server:
    Server::builder()
        .add_service(VmServiceServer::new(service))
        .serve_ext_with_shutdown(options.address, serv_term_rx.map(|_| ()))
        .map(|res| {
            info!("VM server is shutted down");
            res
        })
        .await
        .expect("internal fail");

    Ok(())
}
