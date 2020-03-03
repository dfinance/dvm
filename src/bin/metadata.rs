use std::net::SocketAddr;

use anyhow::Result;
use structopt::StructOpt;
use tonic::transport::Server;
use move_vm_in_cosmos::compiled_protos::vm_grpc::vm_script_metadata_server::VmScriptMetadataServer;
use move_vm_in_cosmos::vm::metadata::MetadataService;

#[derive(Debug, StructOpt, Clone)]
struct Options {
    #[structopt(help = "Address in the form of HOST_ADDRESS:PORT")]
    address: SocketAddr,
}

#[tokio::main]
async fn main() -> Result<()> {
    let address = Options::from_args().address;

    let metadata_service = MetadataService::default();
    Server::builder()
        .add_service(VmScriptMetadataServer::new(metadata_service))
        .serve(address)
        .await?;
    Ok(())
}
