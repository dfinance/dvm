//! Server implementation on tonic & tokio.
//! Run with `LISTEN=[::1]:50051 cargo run --bin server`
use tonic::{transport::Server};

use std::collections::HashMap;
use structopt::StructOpt;

use tonic::{transport::Server, Request, Response, Status};
// TODO: XXX: remove this dep?
use language_e2e_tests::data_store::FakeDataStore;

use move_vm_in_cosmos::{grpc, move_lang};
use grpc::{*, vm_service_server::*};
use std::net::SocketAddr;

struct MoveVmService {
    _inner: move_lang::MoveVm,
}

use move_vm_in_cosmos::{cfg, grpc};
use grpc::vm_service_server::*;
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

    let vm = move_lang::MoveVm::new(Box::new(FakeDataStore::default()));
    let service = MoveVmService { _inner: vm };

    println!("Listening on {}", options.address);

    Server::builder()
        .add_service(VmServiceServer::new(service))
        .serve(options.address)
        .await?;

    Ok(())
}
