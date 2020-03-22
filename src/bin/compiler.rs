use std::net::SocketAddr;

use anyhow::Result;
use structopt::StructOpt;
use tokio::time::Duration;

use dvm_api::tonic;
use tonic::transport::{Server, Uri};

use dvm::cli::config::*;
use dvm::compiled_protos::ds_grpc::ds_service_client::DsServiceClient;
use dvm::compiled_protos::vm_grpc::vm_compiler_server::VmCompilerServer;
use dvm::compiler::mvir::CompilerService;
use dvm::vm::metadata::MetadataService;
use dvm::compiled_protos::vm_grpc::vm_script_metadata_server::VmScriptMetadataServer;

/// Move & Mvir compiler with grpc interface.
#[derive(Debug, StructOpt, Clone)]
struct Options {
    /// Address in the form of HOST_ADDRESS:PORT.
    /// This address will be listen to by compilation server.
    /// Listening localhost by default.
    #[structopt(
        name = "listen address",
        default_value = "[::1]:50053",
        help = "Address in the form of HOST_ADDRESS:PORT"
    )]
    address: SocketAddr,

    /// DataSource Server internet address.
    #[structopt(
        name = "data-source uri",
        env = "DVM_DATA_SOURCE",
        default_value = "http://[::1]:50052"
    )]
    ds: Uri,

    #[structopt(flatten)]
    logging: LoggingOptions,

    #[structopt(flatten)]
    integrations: IntegrationsOptions,
}

#[tokio::main]
async fn main() -> Result<()> {
    let options = Options::from_args();

    match options.integrations.sentry_dsn {
        Some(dsn) => {
            let _init_guard = sentry::init(dsn);
            sentry::integrations::panic::register_panic_handler();
        }
        None => println!("SENTRY_DSN environment variable is not provided, Sentry integration is going to be disabled.")
    }

    let address = options.address;
    let ds_address = options.ds;

    println!("Connecting to ds server...");
    let ds_client = loop {
        match DsServiceClient::connect(ds_address.clone()).await {
            Ok(client) => break client,
            Err(_) => tokio::time::delay_for(Duration::from_secs(1)).await,
        }
    };
    println!("Connected to ds server");

    let compiler_service = CompilerService::new(Box::new(ds_client));
    let metadata_service = MetadataService::default();

    Server::builder()
        .add_service(VmCompilerServer::new(compiler_service))
        .add_service(VmScriptMetadataServer::new(metadata_service))
        .serve(address)
        .await?;
    Ok(())
}
