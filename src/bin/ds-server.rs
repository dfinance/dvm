//! Server implementation on tonic & tokio.
//! Run with `cargo run --bin ds-server "[::1]:50052"`

#[macro_use]
extern crate slice_as_array;

use std::net::SocketAddr;

use language_e2e_tests::account::{Account, AccountData};
use libra_types::access_path::AccessPath;
use libra_types::account_address::AccountAddress;
use structopt::StructOpt;
use tonic::{Request, Response, Status};
use tonic::transport::Server;

use grpc::{*, ds_service_server::*};
use move_vm_in_cosmos::grpc;

#[derive(Debug, StructOpt)]
struct Options {
    #[structopt(help = "Address in the form of HOST_ADDRESS:PORT")]
    address: SocketAddr,
}

pub struct DataSourceService {
    // TODO: add mock data
}

fn new_response(blob: &[u8]) -> Response<DsRawResponse> {
    Response::new(DsRawResponse {
        blob: blob.to_vec(),
        error_code: ds_raw_response::ErrorCode::None as i32,
        error_message: vec![],
    })
}

fn new_error_response(
    error_code: ds_raw_response::ErrorCode,
    error_message: String,
) -> Response<DsRawResponse> {
    Response::new(DsRawResponse {
        blob: vec![],
        error_code: error_code as i32,
        error_message: error_message.into_bytes(),
    })
}

fn is_resource(access_path: &AccessPath) -> bool {
    access_path.path[0] == 1
}

fn get_account_data(balance: u64) -> AccountData {
    let account = Account::new();
    AccountData::with_account(account, balance, 0)
}

#[tonic::async_trait]
impl DsService for DataSourceService {
    #[allow(clippy::transmute_ptr_to_ref)]
    async fn get_raw(
        &self,
        request: Request<DsAccessPath>,
    ) -> Result<Response<DsRawResponse>, Status> {
        let request: DsAccessPath = request.into_inner();
        let addr = slice_as_array::slice_as_array!(&request.address[..], [u8; 32]).unwrap();
        let address = AccountAddress::new(*addr);
        println!("DS Request: {:?}", address.to_string());

        let access_path = AccessPath::new(address, request.path);
        let account_data = get_account_data(1000);
        // if Resource
        if is_resource(&access_path) {
            println!("Access path {}", &access_path);
            return Ok(new_response(
                &account_data.to_resource().simple_serialize().unwrap(),
            ));
        }
        println!("No data for request");
        Err(Status::invalid_argument("No data for request."))
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
