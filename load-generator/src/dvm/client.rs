use http::Uri;
use anyhow::{Error, anyhow};
use dvm_net::api::grpc::vm_grpc::vm_script_executor_client::VmScriptExecutorClient;
use dvm_net::api::grpc::vm_grpc::vm_module_publisher_client::VmModulePublisherClient;
use dvm_net::api::grpc::compiler_grpc::dvm_compiler_client::DvmCompilerClient;
use dvm_net::api::tonic::transport::Channel;
use dvm_net::api::tonic::Request;
use libra::account::AccountAddress;
use libra::ds::{WriteSet, WriteSetMut, WriteOp, AccessPath};
use libra::result::StatusCode;
use dvm_net::api::grpc::vm_grpc::{vm_status::Error as ExcError, VmExecuteScript, VmArgs, StructIdent};
use dvm_net::api::grpc::compiler_grpc::{SourceFiles, CompilationUnit};
use dvm_net::api::grpc::vm_grpc::{VmPublishModule, VmExecuteResponse, VmWriteOp};
use std::convert::TryFrom;

pub struct Client {
    executor: VmScriptExecutorClient<Channel>,
    publisher: VmModulePublisherClient<Channel>,
    compiler: DvmCompilerClient<Channel>,
}

impl Client {
    pub async fn new(uri: Uri) -> Result<Client, Error> {
        let executor = VmScriptExecutorClient::connect(uri.clone()).await?;
        let publisher = VmModulePublisherClient::connect(uri.clone()).await?;
        let compiler = DvmCompilerClient::connect(uri).await?;

        Ok(Client {
            executor,
            publisher,
            compiler,
        })
    }

    pub async fn compile(
        &mut self,
        source: &str,
        address: AccountAddress,
    ) -> Result<Vec<u8>, Error> {
        let files = SourceFiles {
            units: vec![CompilationUnit {
                text: source.to_owned(),
                name: "sources".to_string(),
            }],
            address: address.to_vec(),
        };

        let mut response = self
            .compiler
            .compile(Request::new(files))
            .await?
            .into_inner();

        if !response.errors.is_empty() {
            Err(anyhow!(response.errors.join("\n")))
        } else if response.units.is_empty() {
            Err(anyhow!("Unexpected compiler result"))
        } else {
            Ok(response.units.remove(0).bytecode)
        }
    }

    pub async fn publish(
        &mut self,
        bytecode: Vec<u8>,
        max_gas_amount: u64,
        gas_unit_price: u64,
        address: AccountAddress,
    ) -> Result<ExecutionResult, Error> {
        let request = VmPublishModule {
            sender: address.to_vec(),
            max_gas_amount,
            gas_unit_price,
            code: bytecode,
        };
        ExecutionResult::try_from(
            self.publisher
                .publish_module(Request::new(request))
                .await?
                .into_inner(),
        )
    }

    pub async fn execute(
        &mut self,
        bytecode: Vec<u8>,
        max_gas_amount: u64,
        gas_unit_price: u64,
        senders: Vec<AccountAddress>,
        args: Vec<VmArgs>,
        type_params: Vec<StructIdent>,
    ) -> Result<ExecutionResult, Error> {
        let request = VmExecuteScript {
            senders: senders.into_iter().map(|a| a.to_vec()).collect(),
            max_gas_amount,
            gas_unit_price,
            code: bytecode,
            type_params,
            args,
        };

        ExecutionResult::try_from(
            self.executor
                .execute_script(Request::new(request))
                .await?
                .into_inner(),
        )
    }
}

pub struct ExecutionResult {
    pub ws: WriteSet,
    pub gas_used: u64,
    pub status: StatusCode,
}

impl TryFrom<VmExecuteResponse> for ExecutionResult {
    type Error = Error;

    fn try_from(response: VmExecuteResponse) -> Result<Self, Self::Error> {
        let ws = response
            .write_set
            .into_iter()
            .map(|ws| {
                let op = if VmWriteOp::Value as i32 == ws.r#type {
                    WriteOp::Value(ws.value)
                } else {
                    WriteOp::Deletion
                };

                let path = ws
                    .path
                    .ok_or_else(|| anyhow!("Unexpected access path:None"))
                    .and_then(|ap| {
                        Ok(AccessPath::new(
                            AccountAddress::try_from(ap.address)?,
                            ap.path,
                        ))
                    })?;
                Ok((path, op))
            })
            .collect::<Result<Vec<(AccessPath, WriteOp)>, Error>>()?;
        let ws = WriteSetMut::new(ws).freeze()?;

        let status = response
            .status
            .and_then(|status| status.error)
            .and_then(|status| {
                Some(match status {
                    ExcError::MoveError(code) => StatusCode::try_from(code.status_code).ok()?,
                    ExcError::Abort(_) => StatusCode::ABORTED,
                    ExcError::ExecutionFailure(failure) => {
                        StatusCode::try_from(failure.status_code).ok()?
                    }
                })
            })
            .unwrap_or_else(|| StatusCode::EXECUTED);

        Ok(ExecutionResult {
            ws,
            gas_used: response.gas_used,
            status,
        })
    }
}
