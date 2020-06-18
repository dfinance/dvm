use dvm_net::endpoint::Endpoint;
use dvm_info::config::InfoServiceConfig;
use futures::Future;
use dvm_info::heartbeat::HeartRateMonitor;
use std::time::Duration;
use dvm_info::web::start_info_service;
use dvm_net::api::grpc::vm_grpc::vm_script_executor_client::VmScriptExecutorClient;
use dvm_net::api::tonic::Request;
use dvm_net::api::grpc::vm_grpc::{VmExecuteScript};
use tokio::time::delay_for;
use libra::move_core_types::language_storage::CORE_CODE_ADDRESS;

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
    let mut client = VmScriptExecutorClient::new(connection);

    let request = Request::new(VmExecuteScript {
        address: CORE_CODE_ADDRESS.to_vec(),
        max_gas_amount: 100,
        gas_unit_price: 1,
        code: bytecode,
        type_params: vec![],
        args: vec![],
    });

    client.execute_script(request).await?;
    Ok(())
}
