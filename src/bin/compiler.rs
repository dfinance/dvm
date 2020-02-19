use std::net::SocketAddr;

use anyhow::Result;
use structopt::StructOpt;
use tokio::time::Duration;
use tonic::transport::{Server, Uri};

use move_vm_in_cosmos::compiled_protos::ds_grpc::ds_service_client::DsServiceClient;
use move_vm_in_cosmos::compiled_protos::vm_grpc::vm_compiler_server::VmCompilerServer;
use move_vm_in_cosmos::compiler::mvir::CompilerService;

#[derive(Debug, StructOpt, Clone)]
struct Options {
    #[structopt(help = "Address in the form of HOST_ADDRESS:PORT")]
    address: SocketAddr,
    #[structopt(help = "DataSource Server internet address")]
    ds: Uri,
}

#[tokio::main]
async fn main() -> Result<()> {
    let address = Options::from_args().address;
    let ds_address = Options::from_args().ds;

    println!("Connecting to ds server...");
    let ds_client = loop {
        match DsServiceClient::connect(ds_address.clone()).await {
            Ok(client) => break client,
            Err(_) => tokio::time::delay_for(Duration::from_secs(1)).await,
        }
    };
    println!("Connected to ds server");

    let compiler_service = CompilerService::new(Box::new(ds_client));

    Server::builder()
        .add_service(VmCompilerServer::new(compiler_service))
        .serve(address)
        .await?;
    Ok(())
}
