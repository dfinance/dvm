//! Server implementation on tonic & tokio.

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

unsafe impl Send for MoveVmService {}
unsafe impl Sync for MoveVmService {}

#[tonic::async_trait]
impl VmService for MoveVmService {
    async fn execute_contracts(
        &self,
        request: Request<VmExecuteRequest>,
    ) -> Result<Response<VmExecuteResponses>, Status> {
        println!("Got a request from {:?}", request.remote_addr());

        // TODO: just do some logic here
        let reply = VmExecuteResponses {
            executions: vec![VmExecuteResponse {
                gas_used: 0,
                status: 0,
                status_struct: None,
                events: Vec::default(),
                write_set: HashMap::default(),
            }],
        };
        Ok(Response::new(reply))
    }

    async fn get_imports(
        &self,
        request: Request<VmImportsRequest>,
    ) -> Result<Response<VmImportsResponses>, Status> {
        println!("Got a request from {:?}", request.remote_addr());

        // TODO: just do some logic here
        let reply = VmImportsResponses {
            imports: vec![VmImportsResponse {
                gas_used: 0,
                keys: Vec::default(),
            }],
        };
        Ok(Response::new(reply))
    }
}

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
