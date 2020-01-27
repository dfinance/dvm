//! Server implementation on tonic & tokio.

use structopt::StructOpt;
use http::Uri;
use move_vm_in_cosmos::compiled_protos::vm_grpc::{vm_service_client, VmExecuteRequest};

#[derive(Debug, StructOpt)]
struct Options {
    #[structopt(name = "server_address", help = "Server internet address")]
    server_address: Uri,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = Options::from_args();

    let mut client = vm_service_client::VmServiceClient::connect(options.server_address).await?;
    //  req: execute_contracts
    {
        let request = tonic::Request::new(VmExecuteRequest {
            contracts: Vec::default(),
            options: Default::default(), // u64
        });
        let response = client.execute_contracts(request).await?;
        println!("RESPONSE:\n{:?}", response);
    }

    Ok(())
}
