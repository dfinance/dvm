//! Server implementation on tonic & tokio.
//! Run with `cargo run --bin server "[::1]:50051" "http://[::1]:50052"`
use std::cell::RefCell;
use std::net::SocketAddr;
use std::sync::mpsc;
use std::time::Duration;

#[macro_use]
extern crate log;

use http::Uri;
use libra::libra_logger::try_init_for_testing;
use structopt::StructOpt;
use tokio::runtime::Runtime;

use dvm_api::tonic;
use tonic::transport::{Channel, Server};

use dvm::cli::config::*;
use dvm::ds::view as rds;
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = Options::from_args();
    let _guard = dvm::cli::init(&options.logging, &options.integrations);

    let serv_addr = options.address;
    let ds_addr = options.ds;

    let (tx, rx) = mpsc::channel::<rds::Request>();
    let (rtx, rrx) = mpsc::channel::<rds::Response>();

    let mut runtime = Runtime::new().unwrap();

    {
        let ds = rds::CachingDataSource::new(tx, rrx);
        PriceOracle::new(Box::new(ds.clone())).reg_function();

        // enable logging for libra MoveVM
        // std::env::set_var("RUST_LOG", "warn");
        try_init_for_testing();

        info!("DVM server listening on {}", serv_addr);
        runtime.spawn(async move {
            let service = MoveVmService::new(Box::new(ds)).unwrap();
            Server::builder()
                .add_service(VmServiceServer::new(service))
                .serve(serv_addr)
                .await
        });
    }

    info!("Connecting to data-source: {}", ds_addr);
    let client: RefCell<DsServiceClient<Channel>> = runtime
        .block_on(async {
            loop {
                match DsServiceClient::connect(ds_addr.clone()).await {
                    Ok(client) => return client,
                    Err(_) => tokio::time::delay_for(Duration::from_secs(1)).await,
                }
            }
        })
        .into();
    info!("Connected to data-source");

    // finally looping over channel connected to the proxy data-source:
    // 1. receive request from blocking data-source
    // 2. send asynchronous request to remote data-source
    // 3. send response to blocking data-source, so unblock it.
    rx.iter().for_each(move |ap| {
        let mut client = client.borrow_mut();
        let request = tonic::Request::new(access_path_into_ds(ap));
        runtime.block_on(async {
            let res = client.get_raw(request).await;
            if let Err(ref err) = res {
                dbg!(err);
                return;
            }
            let ds_response = res.unwrap().into_inner();
            if let Err(err) = rtx.send(ds_response) {
                error!("Internal VM-DS channel error: {:?}", err);
                // TODO: Are we should break this loop when res is error?
            }
        });
    });

    unreachable!();
}
