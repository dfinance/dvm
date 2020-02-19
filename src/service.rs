use std::convert::TryFrom;

use anyhow::Error;
use libra_state_view::StateView;
use libra_types::account_address::AccountAddress;
use libra_types::contract_event::ContractEvent;
use libra_types::language_storage::TypeTag;
use libra_types::transaction::{
    Module, parse_as_address, parse_as_bool, parse_as_byte_array, parse_as_u64, Script,
    TransactionStatus,
};
use libra_types::vm_error::{StatusCode, VMStatus};
use libra_types::write_set::{WriteOp, WriteSet};
use tonic::{Request, Response, Status};

use crate::compiled_protos::vm_grpc::{
    ContractType, VmAccessPath, VmContract, VmErrorStatus, VmEvent, VmExecuteRequest,
    VmExecuteResponse, VmExecuteResponses, VmStructTag, VmType, VmTypeTag, VmValue,
};
use crate::compiled_protos::vm_grpc::vm_service_server::VmService;
use crate::ds::MergeWriteSet;
use crate::vm::{bech32_into_libra_address, ExecutionMeta, VM, VmResult};
use crate::vm::ExecutionResult;
use crate::vm::MoveVm;

pub struct MoveVmService {
    vm: MoveVm,
    write_set_handler: Option<Box<dyn MergeWriteSet>>, // Used for auto write change set.
}

unsafe impl Send for MoveVmService {}

unsafe impl Sync for MoveVmService {}

impl MoveVmService {
    pub fn new(view: Box<dyn StateView>) -> Result<MoveVmService, Error> {
        Ok(MoveVmService {
            vm: MoveVm::new(view)?,
            write_set_handler: None,
        })
    }

    pub fn with_auto_commit(
        view: Box<dyn StateView>,
        write_set_handler: Box<dyn MergeWriteSet>,
    ) -> Result<MoveVmService, Error> {
        Ok(MoveVmService {
            vm: MoveVm::new(view)?,
            write_set_handler: Some(write_set_handler),
        })
    }

    pub fn execute_contract(&self, contract: VmContract, _options: u64) -> VmExecuteResponse {
        VmExecuteResponse::from(Contract::try_from(contract).and_then(|contract| {
            let res = match contract.code {
                Code::Module(code) => self.vm.publish_module(contract.meta, code),
                Code::Script(script) => self.vm.execute_script(contract.meta, script),
            };
            //Temporary grpc test case
            if let Some(write_set_handler) = &self.write_set_handler {
                if let Ok(res) = &res {
                    write_set_handler.merge_write_set(res.write_set.clone());
                }
            }
            res
        }))
    }
}

#[tonic::async_trait]
impl VmService for MoveVmService {
    async fn execute_contracts(
        &self,
        request: Request<VmExecuteRequest>,
    ) -> Result<Response<VmExecuteResponses>, Status> {
        let request: VmExecuteRequest = request.into_inner();
        let options = request.options;
        let executions = request
            .contracts
            .into_iter()
            .map(|contract| self.execute_contract(contract, options))
            .collect();
        Ok(Response::new(VmExecuteResponses { executions }))
    }
}

#[derive(Debug)]
struct Contract {
    meta: ExecutionMeta,
    code: Code,
}

#[derive(Debug)]
enum Code {
    Module(Module),
    Script(Script),
}

impl TryFrom<VmContract> for Contract {
    type Error = VMStatus;

    fn try_from(contract: VmContract) -> Result<Self, Self::Error> {
        let address = bech32_into_libra_address(&contract.address).map_err(|_| {
            VMStatus::new(StatusCode::INVALID_DATA).with_message(format!(
                "Invalid AccountAddress: invalid bech32 address {}",
                &contract.address
            ))
        })?;
        let meta = ExecutionMeta::new(
            contract.max_gas_amount,
            contract.gas_unit_price,
            AccountAddress::try_from(address).map_err(|err| {
                VMStatus::new(StatusCode::INVALID_DATA)
                    .with_message(format!("Invalid AccountAddress: {:?}", err))
            })?,
        );

        let code = match ContractType::from_i32(contract.contract_type) {
            Some(ContractType::Module) => Ok(Code::Module(Module::new(contract.code))),
            Some(ContractType::Script) => {
                let args = contract
                    .args
                    .into_iter()
                    .map(|arg| {
                        match VmTypeTag::from_i32(arg.r#type) {
                            Some(VmTypeTag::Bool) => parse_as_bool(&arg.value),
                            Some(VmTypeTag::U64) => parse_as_u64(&arg.value),
                            Some(VmTypeTag::ByteArray) => parse_as_byte_array(&arg.value),
                            Some(VmTypeTag::Address) => {
                                match bech32_into_libra_address(&arg.value) {
                                    Ok(address) => parse_as_address(&format!("0x{}", address)),
                                    Err(_) => Err(Error::msg("Invalid args type.")),
                                }
                            }
                            _ => Err(Error::msg("Invalid args type.")),
                        }
                        .map_err(|err| {
                            VMStatus::new(StatusCode::INVALID_DATA)
                                .with_message(format!("Invalid contract args [{:?}].", err))
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(Code::Script(Script::new(contract.code, args)))
            }
            None => Err(VMStatus::new(StatusCode::INVALID_DATA)
                .with_message("Invalid contract type.".to_string())),
        }?;

        Ok(Contract { meta, code })
    }
}

impl From<VmResult> for VmExecuteResponse {
    fn from(res: Result<ExecutionResult, VMStatus>) -> Self {
        match res {
            Ok(res) => {
                let (status, status_struct) = match res.status {
                    TransactionStatus::Discard(status) => (0, Some(convert_status(status))),
                    TransactionStatus::Keep(status) => (1, Some(convert_status(status))),
                };

                VmExecuteResponse {
                    gas_used: res.gas_used,
                    status,
                    status_struct,
                    events: convert_events(res.events),
                    write_set: convert_write_set(res.write_set),
                }
            }
            Err(err) => {
                // This is't execution error!
                VmExecuteResponse {
                    gas_used: 0,
                    status: 0,
                    status_struct: Some(convert_status(err)),
                    events: vec![],
                    write_set: vec![],
                }
            }
        }
    }
}

fn convert_status(status: VMStatus) -> VmErrorStatus {
    VmErrorStatus {
        major_status: status.major_status as u64,
        sub_status: status.sub_status.map(|status| status as u64).unwrap_or(0),
        message: status.message.unwrap_or_default(),
    }
}

fn convert_events(events: Vec<ContractEvent>) -> Vec<VmEvent> {
    events
        .into_iter()
        .map(|event| VmEvent {
            key: event.key.to_vec(),
            sequence_number: event.sequence_number,
            r#type: Some(convert_type_tag(event.type_tag)),
            event_data: event.event_data,
        })
        .collect()
}

fn convert_type_tag(type_tag: TypeTag) -> VmType {
    let tag = match type_tag {
        TypeTag::Bool => (0, None),
        TypeTag::U64 => (1, None),
        TypeTag::ByteArray => (2, None),
        TypeTag::Address => (3, None),
        TypeTag::Struct(tag) => (
            4,
            Some(VmStructTag {
                address: tag.address.to_vec(),
                module: tag.module.into_string(),
                name: tag.name.into_string(),
                type_params: tag.type_params.into_iter().map(convert_type_tag).collect(),
            }),
        ),
        TypeTag::U8 => (5, None),
        TypeTag::U128 => (6, None),
    };
    VmType {
        tag: tag.0,
        struct_tag: tag.1,
    }
}

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
