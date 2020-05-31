use dvm_net::endpoint::Endpoint;
use dvm_info::config::InfoServiceConfig;
use futures::Future;
use dvm_info::heartbeat::HeartRateMonitor;
use std::time::Duration;
use dvm_info::web::start_info_service;
use dvm_net::api::grpc::vm_grpc::vm_service_client::VmServiceClient;
use dvm_net::api::tonic::Request;
use dvm_net::api::grpc::vm_grpc::{VmExecuteRequest, VmContract, ContractType};
use tokio::time::delay_for;

static TEST_SCRIPT: &str = "script{fun main() {}}";

/// Create and run information service.
pub fn create_info_service(
    dvm_address: Endpoint,
    info_service: InfoServiceConfig,
) -> (Option<impl Future>, Option<HeartRateMonitor>) {
    if let Some(info_service_addr) = info_service.info_service_addr {
        let hrm = HeartRateMonitor::new(Duration::from_secs(info_service.heartbeat_max_interval));
        let bytecode = compiler::compile(TEST_SCRIPT, None).unwrap();
        tokio::spawn(dvm_ping_process(
            dvm_address,
            hrm.clone(),
            Duration::from_secs(info_service.heartbeat_stimulation_interval),
            bytecode,
        ));

        let info_service = start_info_service(
            info_service_addr,
            hrm.clone(),
            Duration::from_secs(info_service.metric_update_interval),
        );
        (Some(info_service), Some(hrm))
    } else {
        (None, None)
    }
}

/// Ping dvm process.
async fn dvm_ping_process(
    endpoint: Endpoint,
    hrm: HeartRateMonitor,
    stimulation_interval: Duration,
    bytecode: Vec<u8>,
) {
    async {
        loop {
            delay_for(Duration::from_secs(1)).await;
            if stimulation_interval < hrm.last_heartbeat_interval() {
                if let Err(err) = send_ping(endpoint.clone(), bytecode.clone()).await {
                    error!("Health check failed:{:?}", err);
                }
            }
        }
    }
    .await
}

/// Send ping request.
async fn send_ping(
    endpoint: Endpoint,
    bytecode: Vec<u8>,
) -> Result<(), Box<dyn std::error::Error>> {
    let connection = endpoint.connect().await?;
    let mut client = VmServiceClient::new(connection);

    let request = Request::new(VmExecuteRequest {
        contracts: vec![VmContract {
            address: "0x0".to_string(),
            max_gas_amount: 100,
            gas_unit_price: 1,
            code: bytecode,
            contract_type: ContractType::Script as i32,
            args: vec![],
        }],
        options: 0,
    });

    client.execute_contracts(request).await?;
    Ok(())
}
