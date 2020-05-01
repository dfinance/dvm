//! Definance Virtual Machine
//! server implementation on tonic & tokio.
//! Run with `cargo run --bin dvm "[::1]:50051" "http://[::1]:50052"`
use std::net::SocketAddr;

#[macro_use]
extern crate log;

use http::Uri;
use libra::libra_logger::init_struct_log_from_env;
use structopt::StructOpt;

use dvm_api::tonic;
use tonic::transport::Server;

use dvm_api::grpc::vm_grpc::vm_service_server::VmServiceServer;
use data_source::{GrpcDataSource, ModuleCache};
use anyhow::Result;
use services::vm::VmService;
use dvm_cli::config::*;
use dvm_cli::logging;

const MODULE_CACHE: usize = 1000;

/// Definance Virtual Machine with gRPC interface.
///
/// API described in protobuf schemas: https://github.com/dfinance/dvm-proto
#[derive(Debug, StructOpt, Clone)]
struct Options {
    /// Address in the form of HOST_ADDRESS:PORT.
    /// The address will be listen to by DVM (this) server.
    /// Listening localhost by default.
    #[structopt(
        name = "listen address",
        default_value = "[::1]:50051",
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
    let _guard = logging::init(&options.logging, &options.integrations);
    main_internal(options)
}

#[tokio::main]
async fn main_internal(options: Options) -> Result<()> {
    let ds = GrpcDataSource::new(options.ds).expect("Unable to instantiate GrpcDataSource.");
    let ds = ModuleCache::new(ds, MODULE_CACHE);

    init_struct_log_from_env().unwrap();
    info!("DVM server listening on {}", options.address);
    let service = VmService::new(ds).expect("Unable to initialize VmService.");
    Server::builder()
        .add_service(VmServiceServer::new(service))
        .serve(options.address)
        .await?;
    Ok(())
}