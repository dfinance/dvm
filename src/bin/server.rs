//! Server implementation on tonic & tokio.
//! Run with `LISTEN=[::1]:50051 cargo run --bin server`
use tonic::{transport::Server};

use move_vm_in_cosmos::{cfg, grpc};
use grpc::vm_service_server::*;
use move_vm_in_cosmos::ds::MockDataSource;
use move_vm_in_cosmos::service::MoveVmService;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = cfg::env::get_cfg_vars().into_sock_addr()?;
    let service = MoveVmService::new(Box::new(MockDataSource::default()));

    println!("{:?} listening on {1}", cfg.name, cfg.address);

    Server::builder()
        .add_service(VmServiceServer::new(service))
        .serve(cfg.address)
        .await?;

    Ok(())
}
