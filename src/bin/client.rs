//! Server implementation on tonic & tokio.
//! Run with `LISTEN="http://[::1]:50051" cargo run --bin client`

use move_vm_in_cosmos::{cfg, grpc};
use grpc::{*, vm_service_client::*};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = cfg::env::get_cfg_vars();

    let mut client = VmServiceClient::connect(cfg.address.clone()).await?;
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
