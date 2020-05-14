use anyhow::Result;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use dvm_net::tonic::transport::Channel;
use dvm_net::tonic::Request;
use crate::ArcMut;
use crate::compiled_protos::vm_grpc::vm_service_client::VmServiceClient;
use crate::compiled_protos::vm_grpc::{VmExecuteRequest, VmExecuteResponses};

pub struct Client {
    runtime: ArcMut<Runtime>,
    client: ArcMut<VmServiceClient<Channel>>,
}

impl Client {
    pub fn new(port: u32) -> Result<Client> {
        let mut runtime = Runtime::new().unwrap();
        let client = runtime.block_on(async {
            VmServiceClient::connect(format!("http://localhost:{}", port)).await
        })?;

        let client = Client {
            runtime: Arc::new(Mutex::new(runtime)),
            client: Arc::new(Mutex::new(client)),
        };
        Ok(client)
    }

    pub fn perform_request(&self, request: Request<VmExecuteRequest>) -> VmExecuteResponses {
        let mut rt = self.runtime.lock().unwrap();
        let client = self.client.clone();
        rt.block_on(async {
            client
                .lock()
                .unwrap()
                .execute_contracts(request)
                .await
                .unwrap()
        })
        .into_inner()
    }
}
