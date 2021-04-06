use anyhow::Result;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use dvm_net::tonic::transport::Channel;
use dvm_net::tonic::Request;
use crate::ArcMut;
use crate::compiled_protos::vm_script_executor_client::VmScriptExecutorClient;
use crate::compiled_protos::vm_module_publisher_client::VmModulePublisherClient;
use crate::compiled_protos::{VmPublishModule, VmExecuteResponse};
use dvm_net::api::grpc::VmExecuteScript;

/// Vm Grpc client.
pub struct Client {
    runtime: ArcMut<Runtime>,
    executor: ArcMut<VmScriptExecutorClient<Channel>>,
    publisher: ArcMut<VmModulePublisherClient<Channel>>,
}

impl Client {
    #[allow(clippy::eval_order_dependence)]
    /// Creates a new grpc client with service port.
    pub fn new(port: u32) -> Result<Client> {
        let mut runtime = Runtime::new().unwrap();
        let (executor, publisher) = runtime.block_on(async {
            (
                VmScriptExecutorClient::connect(format!("http://localhost:{}", port)).await,
                VmModulePublisherClient::connect(format!("http://localhost:{}", port)).await,
            )
        });

        let client = Client {
            runtime: Arc::new(Mutex::new(runtime)),
            executor: Arc::new(Mutex::new(executor?)),
            publisher: Arc::new(Mutex::new(publisher?)),
        };
        Ok(client)
    }

    /// Publish module.
    pub fn publish_module(&self, request: VmPublishModule) -> VmExecuteResponse {
        let mut rt = self.runtime.lock().unwrap();
        let client = self.publisher.clone();
        rt.block_on(async {
            client
                .lock()
                .unwrap()
                .publish_module(Request::new(request))
                .await
                .unwrap()
        })
        .into_inner()
    }

    /// Execute script.
    pub fn execute_script(&self, request: VmExecuteScript) -> VmExecuteResponse {
        let mut rt = self.runtime.lock().unwrap();
        let client = self.executor.clone();
        rt.block_on(async {
            client
                .lock()
                .unwrap()
                .execute_script(Request::new(request))
                .await
                .unwrap()
        })
        .into_inner()
    }
}
