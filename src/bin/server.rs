//! Server implementation on tonic & tokio.
//! Run with `cargo run --bin server "[::1]:50051" "http://[::1]:50052"`
use std::sync::Mutex;
use std::sync::Arc;
use std::net::SocketAddr;
use http::Uri;
use structopt::StructOpt;

use tokio::runtime::Runtime;
use tonic::transport::Server;

use move_vm_in_cosmos::grpc;
use move_vm_in_cosmos::ds::GrpcDataSource;
use move_vm_in_cosmos::ds::MockDataSource;
use move_vm_in_cosmos::service::MoveVmService;
use grpc::vm_service_server::*;

#[derive(Debug, StructOpt, Clone)]
struct Options {
    #[structopt(help = "Address in the form of HOST_ADDRESS:PORT")]
    address: SocketAddr,

    #[structopt(help = "DataSource Server internet address")]
    ds: Uri,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = Arc::new(Mutex::new(Runtime::new()?));

    let options = Options::from_args();

    let ws = MockDataSource::default();

    let ds = {
        let client = {
            println!("Connecting to data-source: {}", options.ds);

            let ds_uri = options.ds;
            use crate::grpc::ds_service_client::DsServiceClient;
            let mut runtime = runtime.lock().unwrap();
            runtime.block_on(async { DsServiceClient::connect(ds_uri).await })?
        };
        GrpcDataSource::new_with(Arc::clone(&runtime), client)
    };

    let service = MoveVmService::with_auto_commit(Box::new(ds), Box::new(ws));

    println!("Listening on {}", options.address);
    let bind_addr = options.address;
    runtime.lock().unwrap().block_on(async move {
        Server::builder()
            .add_service(VmServiceServer::new(service))
            .serve(bind_addr)
            .await
    })?;

    Ok(())
}
