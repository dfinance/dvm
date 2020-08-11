use libra::{prelude::*, vm::*};
use std::sync::Arc;
use data_source::DataSource;
use info::heartbeat::HeartRateMonitor;
use crate::{tonic, api};
use tonic::{Request, Response, Status};
use api::grpc::vm_grpc::vm_script_executor_server::VmScriptExecutor;
use dvm_net::api::grpc::vm_grpc::*;
use runtime::move_vm::{ExecutionMeta, Script, ExecutionResult, Dvm};
use std::convert::TryFrom;
use anyhow::Error;
use byteorder::{LittleEndian, ByteOrder};
use info::metrics::meter::ScopeMeter;
use info::metrics::execution::ExecutionResult as ActionResult;
use dvm_net::api::grpc::vm_grpc::vm_module_publisher_server::VmModulePublisher;

/// Virtual machine service.
#[derive(Clone)]
pub struct VmService<D: DataSource> {
    vm: Arc<Dvm<D>>,
    hrm: Arc<Option<HeartRateMonitor>>,
}

unsafe impl<D> Send for VmService<D> where D: DataSource {}

unsafe impl<D> Sync for VmService<D> where D: DataSource {}

impl<D> VmService<D>
where
    D: DataSource,
{
    /// Creates a new virtual machine service with the given data source and request interval counter.
    pub fn new(view: D, hrm: Option<HeartRateMonitor>) -> VmService<D> {
        VmService {
            vm: Arc::new(Dvm::new(view)),
            hrm: Arc::new(hrm),
        }
    }
}

#[tonic::async_trait]
impl<D> VmScriptExecutor for VmService<D>
where
    D: DataSource,
{
    async fn execute_script(
        &self,
        request: Request<VmExecuteScript>,
    ) -> Result<Response<VmExecuteResponse>, Status> {
        let meter = ScopeMeter::new("execute_script");
        let request = request.into_inner();
        let response = ExecuteScript::try_from(request)
            .map_err(|err| {
                PartialVMError::new(StatusCode::INVALID_DATA)
                    .with_message(format!("Invalid contract args [{:?}].", err))
                    .finish(Location::Undefined)
                    .into_vm_status()
            })
            .and_then(|contract| self.vm.execute_script(contract.meta, contract.script));
        Ok(Response::new(store_metric(
            vm_result_to_execute_response(response),
            meter,
        )))
    }
}

/// Converts execution result to api response.
fn vm_result_to_execute_response(res: Result<ExecutionResult, VMStatus>) -> VmExecuteResponse {
    match res {
        Ok(res) => VmExecuteResponse {
            gas_used: res.gas_used,
            events: convert_events(res.events),
            write_set: convert_write_set(res.write_set),
            status: Some(convert_vm_error_status(res.status)),
        },
        Err(err) => {
            // This is't execution error!
            VmExecuteResponse {
                gas_used: 0,
                events: vec![],
                write_set: vec![],
                status: Some(convert_status(err, None)),
            }
        }
    }
}

/// Converts libra `VmError` into gRPC `VMStatus`.
fn convert_vm_error_status(status: VMError) -> VmStatus {
    let msg = status.message().map(|m| m.to_owned());
    convert_status(status.into_vm_status(), msg)
}

/// Converts libra `VMStatus` into gRPC `VMStatus`.
fn convert_status(status: VMStatus, message: Option<String>) -> VmStatus {
    let message = message.map(|msg| Message { text: msg });
    match status {
        VMStatus::Executed => VmStatus {
            error: None,
            message,
        },
        VMStatus::Error(status_code) => VmStatus {
            error: Some(vm_status::Error::MoveError(MoveError {
                status_code: status_code as u64,
            })),
            message,
        },
        VMStatus::MoveAbort(loc, abort_code) => VmStatus {
            error: Some(vm_status::Error::Abort(Abort {
                abort_location: convert_abort_location(loc),
                abort_code,
            })),
            message,
        },
        VMStatus::ExecutionFailure {
            status_code,
            location,
            function,
            code_offset,
        } => VmStatus {
            error: Some(vm_status::Error::ExecutionFailure(Failure {
                status_code: status_code as u64,
                abort_location: convert_abort_location(location),
                function_loc: Some(FunctionLoc {
                    function: function as u64,
                    code_offset: code_offset as u64,
                }),
            })),
            message,
        },
    }
}

fn convert_abort_location(location: AbortLoc) -> Option<AbortLocation> {
    match location {
        AbortLoc::Module(module) => Some(AbortLocation {
            address: module.address().to_vec(),
            module: module.name().to_string(),
        }),
        AbortLoc::Script => None,
    }
}

/// Converts `WriteSet`.into a vec of `VmValue` for gRPC
fn convert_write_set(ws: WriteSet) -> Vec<VmValue> {
    ws.into_iter()
        .map(|(access_path, write_op)| {
            let path = Some(VmAccessPath {
                address: access_path.address.to_vec(),
                path: access_path.path,
            });
            match write_op {
                WriteOp::Value(blob) => VmValue {
                    r#type: 0,
                    value: blob,
                    path,
                },
                WriteOp::Deletion => VmValue {
                    r#type: 1,
                    value: vec![],
                    path,
                },
            }
        })
        .collect()
}

/// Converts libra `ContractEvent`s into gRPC `VmEvent`.
fn convert_events(events: Vec<ContractEvent>) -> Vec<VmEvent> {
    events
        .into_iter()
        .map(|event| match event {
            ContractEvent::V0(event) => {
                let event_type = Some(convert_event_tag(event.type_tag()));
                VmEvent {
                    sender_address: event.key.get_creator_address().to_vec(),
                    event_data: event.event_data,
                    event_type,
                    sender_module: event.caller_module.map(|id| ModuleIdent {
                        address: id.address().to_vec(),
                        name: id.name().as_str().to_owned(),
                    }),
                }
            }
        })
        .collect()
}

/// Converts event type tag.
fn convert_event_tag(type_tag: &TypeTag) -> LcsTag {
    fn tag(
        type_tag: LcsType,
        vector_type: Option<LcsTag>,
        struct_ident: Option<StructIdent>,
    ) -> LcsTag {
        LcsTag {
            type_tag: type_tag as i32,
            vector_type: vector_type.map(Box::new),
            struct_ident,
        }
    }

    match type_tag {
        TypeTag::Bool => tag(LcsType::LcsBool, None, None),
        TypeTag::U64 => tag(LcsType::LcsU64, None, None),
        TypeTag::Vector(v) => tag(LcsType::LcsVector, Some(convert_event_tag(v)), None),
        TypeTag::Address => tag(LcsType::LcsAddress, None, None),
        TypeTag::Struct(t) => tag(
            LcsType::LcsStruct,
            None,
            Some(StructIdent {
                address: t.address.to_vec(),
                module: t.module.as_str().to_owned(),
                name: t.name.as_str().to_owned(),
                type_params: t.type_params.iter().map(convert_event_tag).collect(),
            }),
        ),
        TypeTag::U8 => tag(LcsType::LcsU8, None, None),
        TypeTag::U128 => tag(LcsType::LcsU128, None, None),
        TypeTag::Signer => tag(LcsType::LcsSigner, None, None),
    }
}

/// Store execution result to 'scope_meter'.
fn store_metric(result: VmExecuteResponse, mut scope_meter: ScopeMeter) -> VmExecuteResponse {
    let status = match &result.status {
        Some(status) => match &status.error {
            Some(vm_status::Error::Abort(_)) => StatusCode::ABORTED as u64,
            Some(vm_status::Error::MoveError(error)) => error.status_code,
            Some(vm_status::Error::ExecutionFailure(failure)) => failure.status_code,
            None => StatusCode::EXECUTED as u64,
        },
        None => 0,
    };

    scope_meter.set_result(ActionResult::new(
        result
            .status
            .as_ref()
            .map(|status| status.error.is_none())
            .unwrap_or(false),
        status,
        result.gas_used,
    ));

    result
}

/// Data for script execution.
#[derive(Debug)]
struct ExecuteScript {
    meta: ExecutionMeta,
    script: Script,
}

impl TryFrom<VmExecuteScript> for ExecuteScript {
    type Error = Error;

    fn try_from(req: VmExecuteScript) -> Result<Self, Error> {
        let args = req
            .args
            .into_iter()
            .map(|arg| {
                let value = arg.value;
                let type_tag =
                    VmTypeTag::from_i32(arg.r#type).ok_or_else(|| anyhow!("Invalid args type."))?;
                Ok(match type_tag {
                    VmTypeTag::Bool => {
                        ensure!(
                            value.len() == 1,
                            "Invalid boolean argument length. Expected 1 byte."
                        );
                        Value::bool(value[0] != 0x0)
                    }
                    VmTypeTag::U64 => {
                        ensure!(
                            value.len() == 8,
                            "Invalid u64 argument length. Expected 8 byte."
                        );
                        Value::u64(LittleEndian::read_u64(&value))
                    }
                    VmTypeTag::Vector => Value::vector_u8(value),
                    VmTypeTag::Address => Value::address(AccountAddress::try_from(value)?),
                    VmTypeTag::U8 => {
                        ensure!(
                            value.len() == 1,
                            "Invalid u8 argument length. Expected 1 byte."
                        );
                        Value::u8(value[0] as u8)
                    }
                    VmTypeTag::U128 => {
                        ensure!(
                            value.len() == 16,
                            "Invalid u64 argument length. Expected 16 byte."
                        );
                        Value::u128(LittleEndian::read_u128(&value))
                    }
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        fn tag(t: LcsTag) -> Result<TypeTag, Error> {
            let type_tag =
                LcsType::from_i32(t.type_tag).ok_or_else(|| anyhow!("Invalid type tag."))?;
            Ok(match type_tag {
                LcsType::LcsBool => TypeTag::Bool,
                LcsType::LcsU64 => TypeTag::U64,
                LcsType::LcsVector => TypeTag::Vector(
                    tag(t
                        .vector_type
                        .map(|t| *t)
                        .ok_or_else(|| anyhow!("Invalid vector tag."))?)
                    .map(Box::new)?,
                ),
                LcsType::LcsAddress => TypeTag::Address,
                LcsType::LcsU8 => TypeTag::U8,
                LcsType::LcsU128 => TypeTag::U128,
                LcsType::LcsSigner => TypeTag::Signer,
                LcsType::LcsStruct => TypeTag::Struct(struct_tag(
                    t.struct_ident
                        .ok_or_else(|| anyhow!("Invalid struct tag."))?,
                )?),
            })
        }

        fn struct_tag(ident: StructIdent) -> Result<StructTag, Error> {
            Ok(StructTag {
                address: AccountAddress::try_from(ident.address)?,
                module: Identifier::new(ident.module)?,
                name: Identifier::new(ident.name)?,
                type_params: ident
                    .type_params
                    .into_iter()
                    .map(tag)
                    .collect::<Result<Vec<TypeTag>, Error>>()?,
            })
        }

        let type_args = req
            .type_params
            .into_iter()
            .map(|ident| Ok(TypeTag::Struct(struct_tag(ident)?)))
            .collect::<Result<Vec<TypeTag>, Error>>()?;

        Ok(ExecuteScript {
            meta: ExecutionMeta::new(
                req.max_gas_amount,
                req.gas_unit_price,
                AccountAddress::try_from(req.address)?,
            )?,
            script: Script::new(req.code, args, type_args),
        })
    }
}

#[tonic::async_trait]
impl<D> VmModulePublisher for VmService<D>
where
    D: DataSource,
{
    async fn publish_module(
        &self,
        request: Request<VmPublishModule>,
    ) -> Result<Response<VmExecuteResponse>, Status> {
        let meter = ScopeMeter::new("publish_module");
        let request = request.into_inner();
        let response = PublishModule::try_from(request)
            .map_err(|err| {
                PartialVMError::new(StatusCode::INVALID_DATA)
                    .with_message(format!("Invalid publish module args [{:?}].", err))
                    .finish(Location::Undefined)
                    .into_vm_status()
            })
            .and_then(|contract| self.vm.publish_module(contract.meta, contract.module));
        Ok(Response::new(store_metric(
            vm_result_to_execute_response(response),
            meter,
        )))
    }
}

/// Data for module publication.
#[derive(Debug)]
struct PublishModule {
    meta: ExecutionMeta,
    module: Module,
}

impl TryFrom<VmPublishModule> for PublishModule {
    type Error = Error;

    fn try_from(request: VmPublishModule) -> Result<Self, Self::Error> {
        Ok(PublishModule {
            meta: ExecutionMeta::new(
                request.max_gas_amount,
                request.gas_unit_price,
                AccountAddress::try_from(request.address)?,
            )?,
            module: Module::new(request.code),
        })
    }
}
