//! Server implementation on tonic & tokio.
//! Run with `cargo run --bin ds-server "[::1]:50052"`

#[macro_use]
extern crate slice_as_array;

use std::net::SocketAddr;
use structopt::StructOpt;

use libra::{libra_types, vm_runtime_types, language_e2e_tests};
use language_e2e_tests::account::{Account, AccountData};
use libra_types::access_path::AccessPath;
use libra_types::account_address::AccountAddress;
use vm_runtime_types::values::Struct;

use dvm_api::tonic;
use tonic::{Request, Response, Status};
use tonic::transport::Server;

use dvm::compiled_protos::ds_grpc::{DsRawResponse, DsAccessPath, DsAccessPaths, DsRawResponses};
use dvm::compiled_protos::ds_grpc::ds_service_server::{DsService, DsServiceServer};

#[derive(Debug, StructOpt)]
struct Options {
    #[structopt(help = "Address in the form of HOST_ADDRESS:PORT")]
    address: SocketAddr,
}

pub struct DataSourceService {
    // TODO: add mock data
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
            let ds_response = DsRawResponse::with_blob(
                &account_data
                    .to_resource()
                    .value_as::<Struct>()
                    .unwrap()
                    .simple_serialize(&AccountData::layout())
                    .unwrap(),
            );
            return Ok(Response::new(ds_response));
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
