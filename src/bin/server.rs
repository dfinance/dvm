//! Server implementation on tonic & tokio.
//! Run with `cargo run --bin server "[::1]:50051" "http://[::1]:50052"`
use std::sync::mpsc;
use std::net::SocketAddr;
use std::cell::RefCell;
use structopt::StructOpt;
use http::Uri;

use tokio::runtime::Runtime;
use tonic::transport::{Server, Channel};

use move_vm_in_cosmos::grpc::vm_service_server::*;
use move_vm_in_cosmos::grpc::ds_service_client::DsServiceClient;
use move_vm_in_cosmos::ds::MockDataSource;
use move_vm_in_cosmos::ds::view as rds;
use move_vm_in_cosmos::service::MoveVmService;

#[derive(Debug, StructOpt, Clone)]
struct Options {
    #[structopt(help = "Address in the form of HOST_ADDRESS:PORT")]
    address: SocketAddr,

    #[structopt(help = "DataSource Server internet address")]
    ds: Uri,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, rx) = mpsc::channel::<rds::Request>();
    let (rtx, rrx) = mpsc::channel::<rds::Response>();

    let options = Options::from_args();
    let serv_addr = options.address;
    let ds_addr = options.ds;

    let mut runtime = Runtime::new().unwrap();

    {
        let ws = MockDataSource::default();
        let ds = rds::CachingDataSource::new(tx, rrx);

        let service = MoveVmService::with_auto_commit(Box::new(ds), Box::new(ws));

        println!("VM server listening on {}", serv_addr);
        runtime.spawn(async move {
            Server::builder()
                .add_service(VmServiceServer::new(service))
                .serve(serv_addr)
                .await
        });
    }

    println!("Connecting to data-source: {}", ds_addr);
    let client: RefCell<DsServiceClient<Channel>> = runtime
        .block_on(async { DsServiceClient::connect(ds_addr).await })
        .expect("Cannot connect to data-source server")
        .into();
    println!("Connected to data-source");

    // finally looping over channel connected to the proxy data-source:
    // 1. receive request from blocking data-source
    // 2. send asynchronous request to remote data-source
    // 3. send response to blocking data-source, so unblock it.
    rx.iter().for_each(move |ap| {
        let mut client = client.borrow_mut();
        let request = tonic::Request::new(ap.into());
        runtime.block_on(async {
            let res = client.get_raw(request).await;
            let res = rtx.send(res.map(|resp| resp.into_inner().blob).ok());
            if let Err(err) = res {
                eprintln!("ERR: Internal VM-DS channel error: {:?}", err);
                // TODO: Are we should break this loop when res is error?
            }
        });
    });

    unreachable!();
}
