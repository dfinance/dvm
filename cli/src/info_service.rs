use dvm_net::endpoint::Endpoint;
use dvm_info::config::InfoServiceConfig;
use futures::Future;
use dvm_info::heartbeat::HeartRateMonitor;
use std::time::Duration;
use dvm_info::web::start_info_service;
use dvm_net::api::grpc::vm_script_executor_client::VmScriptExecutorClient;
use dvm_net::api::tonic::Request;
use dvm_net::api::grpc::VmExecuteScript;
use tokio::time::delay_for;
use libra::prelude::*;

static TEST_SCRIPT: &str = "script{fun main() {}}";

/// Create and run information service.
pub fn create_info_service(
    info_service: InfoServiceConfig,
) -> (Option<impl Future>, Option<HeartRateMonitor>) {
    if let Some(info_service_addr) = info_service.info_service_addr {
        info!("Start info service: {}", info_service_addr);
        let hrm = HeartRateMonitor::new(Duration::from_secs(info_service.heartbeat_max_interval));
        let bytecode = compiler::compile(TEST_SCRIPT, None).unwrap();
        if let Some(dvm_self_addr) = info_service.dvm_self_check_addr {
            info!("Start health check service.");
            tokio::spawn(dvm_ping_process(
                dvm_self_addr,
                hrm.clone(),
                Duration::from_secs(info_service.heartbeat_stimulation_interval),
                bytecode,
            ));
        } else {
            warn!(
                "Health check service is not running, because dvm-self-check-addr is not defined."
            );
        }

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
            if stimulation_interval < hrm.time_since_last_heartbeat() {
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
        senders: vec![CORE_CODE_ADDRESS.to_vec()],
        max_gas_amount: 100,
        gas_unit_price: 1,
        block: 0,
        timestamp: 0,
        code: bytecode,
        type_params: vec![],
        args: vec![],
    });

    client.execute_script(request).await?;
    Ok(())
}
