//! Server implementation on tonic & tokio.
//! Run with `LISTEN=[::1]:50051 cargo run --bin server`

use structopt::StructOpt;

use tonic::transport::Server;

use move_vm_in_cosmos::grpc;
use grpc::vm_service_server::*;
use std::net::SocketAddr;

use move_vm_in_cosmos::ds::MockDataSource;
use move_vm_in_cosmos::service::MoveVmService;

#[derive(Debug, StructOpt)]
struct Options {
    #[structopt(help = "Address in the form of HOST_ADDRESS:PORT")]
    address: SocketAddr,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = Options::from_args();

    let service = MoveVmService::new(Box::new(MockDataSource::default()));

    println!("Listening on {}", options.address);

    Server::builder()
        .add_service(VmServiceServer::new(service))
        .serve(options.address)
        .await?;

    Ok(())
}
