//! Server implementation on tonic & tokio.
//! Run with `cargo run --bin server "[::1]:50051" "http://[::1]:50052"`
use std::cell::RefCell;
use std::net::SocketAddr;
use std::sync::mpsc;
use std::time::Duration;

use http::Uri;
use libra::libra_logger::try_init_for_testing;
use structopt::StructOpt;
use tokio::runtime::Runtime;

use dvm_api::tonic;
use tonic::transport::{Channel, Server};

use dvm::cli::config::*;
use dvm::service::MoveVmService;
use dvm::compiled_protos::access_path_into_ds;
use dvm::compiled_protos::vm_grpc::vm_service_server::VmServiceServer;
use dvm::compiled_protos::ds_grpc::ds_service_client::DsServiceClient;
use dvm::vm::native::{oracle::PriceOracle, Reg};

/// Definance Virtual Machine with gRPC interface.
///
/// API described in protobuf schemas: https://github.com/dfinance/dvm-proto
#[derive(Debug, StructOpt, Clone)]
struct Options {
    /// Address in the form of HOST_ADDRESS:PORT.
    /// This address will be listen to by DVM (this) server.
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
        env = "DVM_DATA_SOURCE",
        default_value = "http://[::1]:50052"
    )]
    ds: Uri,

    #[structopt(flatten)]
    logging: LoggingOptions,

    #[structopt(flatten)]
    integrations: IntegrationsOptions,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = Options::from_args();

    match options.integrations.sentry_dsn {
        Some(dsn) => {
            let _init_guard = sentry::init(dsn);
            sentry::integrations::panic::register_panic_handler();
        }
        None => println!("SENTRY_DSN environment variable is not provided, Sentry integration is going to be disabled.")
    }

    let serv_addr = options.address;
    let ds_addr = options.ds;

    let (tx, rx) = mpsc::channel::<rds::Request>();
    let (rtx, rrx) = mpsc::channel::<rds::Response>();

    let mut runtime = Runtime::new().unwrap();

    {
        let ds = rds::CachingDataSource::new(tx, rrx);
        PriceOracle::new(Box::new(ds.clone())).reg_function();

        // enable logging for libra MoveVM
        std::env::set_var("RUST_LOG", "warn");
        try_init_for_testing();

        println!("VM server listening on {}", serv_addr);
        runtime.spawn(async move {
            let service = MoveVmService::new(Box::new(ds)).unwrap();
            Server::builder()
                .add_service(VmServiceServer::new(service))
                .serve(serv_addr)
                .await
        });

}
