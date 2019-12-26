//! Server implementation on tonic & tokio.
//! Run with `LISTEN="http://[::1]:50051" cargo run --bin client`

use std::collections::HashMap;
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
            imports: HashMap::default(),
            options: Default::default(), // u64
            values: Default::default(),
        });
        let response = client.execute_contracts(request).await?;
        println!("RESPONSE:\n{:?}", response);
    }

    //  req: get_imports
    {
        let request = tonic::Request::new(VmImportsRequest {
            contracts: Vec::default(),
        });
        let response = client.get_imports(request).await?;
        println!("RESPONSE:\n{:?}", response);
    }

    Ok(())
}
