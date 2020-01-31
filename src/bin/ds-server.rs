//! Server implementation on tonic & tokio.
//! Run with `cargo run --bin ds-server "[::1]:50052"`

use std::net::SocketAddr;

use structopt::StructOpt;
use tonic::{Request, Response, Status};
use tonic::transport::Server;

use move_vm_in_cosmos::compiled_protos::ds_grpc::{
    DsAccessPath, DsAccessPaths, DsRawResponse, DsRawResponses,
};
use move_vm_in_cosmos::compiled_protos::ds_grpc::ds_service_server::{DsService, DsServiceServer};
use move_vm_in_cosmos::grpc;

#[derive(Debug, StructOpt)]
struct Options {
    #[structopt(help = "Address in the form of HOST_ADDRESS:PORT")]
    address: SocketAddr,
}

pub struct DataSourceService {
    // TODO: add mock data
}

#[tonic::async_trait]
impl DsService for DataSourceService {
    async fn get_raw(
        &self,
        request: Request<DsAccessPath>,
    ) -> Result<Response<DsRawResponse>, Status> {
        dbg!(format!("get_raw {:?}", &request));
        let request: DsAccessPath = request.into_inner();
        let (_addr, _path) = (request.address, &request.path[..]);
        let found = true;

        if found {
            Ok(Response::new(DsRawResponse {
                blob: Vec::with_capacity(0),
            }))
        } else {
            Err(Status::invalid_argument("No data for request."))
        }
    }

    async fn multi_get_raw(
        &self,
        _request: Request<DsAccessPaths>,
    ) -> Result<Response<DsRawResponses>, Status> {
        Err(Status::invalid_argument("method not implemented."))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = Options::from_args();
    println!("Listening on {}", options.address);

    Server::builder()
        .add_service(DsServiceServer::new(DataSourceService {}))
        .serve(options.address)
        .await?;

    Ok(())
}
