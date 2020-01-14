use tonic::{Request, Response, Status};

use crate::{grpc, move_lang::MoveVm};
use grpc::{*, vm_service_server::*};
use libra_state_view::StateView;
use crate::move_lang::{VM, ExecutionMeta};
use libra_types::account_address::AccountAddress;
use std::convert::TryFrom;
use anyhow::Error;
use libra_types::transaction::{
    Module, Script, parse_as_bool, parse_as_u64, parse_as_byte_array, parse_as_address,
    TransactionStatus,
};
use libra_types::vm_error::{StatusCode, VMStatus};
use libra_types::write_set::{WriteSet, WriteOp};
use libra_types::contract_event::ContractEvent;
use libra_types::language_storage::TypeTag;

pub struct MoveVmService {
    vm: MoveVm,
}

unsafe impl Send for MoveVmService {}

unsafe impl Sync for MoveVmService {}

impl MoveVmService {
    pub fn new(view: Box<dyn StateView>) -> MoveVmService {
        MoveVmService {
            vm: MoveVm::new(view),
        }
    }

    pub fn execute_contract(&self, contract: VmContract, _options: u64) -> VmExecuteResponse {
        let vm_output = Contract::try_from(contract).and_then(|contract| match contract.code {
            Code::Module(code) => self.vm.publish_module(contract.meta, code),
            Code::Script(script) => self.vm.execute_script(contract.meta, script),
        });
        match vm_output {
            Ok(output) => {
                let (status, status_struct) = match output.status().clone() {
                    TransactionStatus::Discard(status) => (0, Some(convert_status(status))),
                    TransactionStatus::Keep(_) => (1, None),
                };
                output.status().vm_status();
                VmExecuteResponse {
                    gas_used: output.gas_used(),
                    status,
                    status_struct,
                    events: convert_events(output.events()),
                    write_set: convert_write_set(output.write_set()),
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

fn convert_status(status: VMStatus) -> VmErrorStatus {
    VmErrorStatus {
        major_status: status.major_status as u64,
        sub_status: status.sub_status.map(|status| status as u64).unwrap_or(0),
        message: status.message.unwrap_or_default(),
    }
}

fn convert_events(events: &[ContractEvent]) -> Vec<VmEvent> {
    events
        .iter()
        .map(|event| VmEvent {
            key: event.key().to_vec(),
            sequence_number: event.sequence_number(),
            r#type: Some(convert_type_tag(event.type_tag())),
            event_data: event.event_data().to_vec(),
        })
        .collect()
}

fn convert_type_tag(type_tag: &TypeTag) -> VmType {
    let tag = match type_tag {
        TypeTag::Bool => (0, None),
        TypeTag::U64 => (1, None),
        TypeTag::ByteArray => (2, None),
        TypeTag::Address => (3, None),
        TypeTag::Struct(tag) => (
            4,
            Some(VmStructTag {
                address: tag.address.to_vec(),
                module: tag.module.as_str().to_owned(),
                name: tag.name.as_str().to_owned(),
                type_params: tag
                    .type_params
                    .iter()
                    .map(|tag| convert_type_tag(tag))
                    .collect(),
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

fn convert_write_set(ws: &WriteSet) -> Vec<VmValue> {
    ws.iter()
        .map(|(access_path, write_op)| {
            let path = Some(VmAccessPath {
                address: access_path.address.to_vec(),
                path: access_path.path.clone(),
            });
            match write_op {
                WriteOp::Value(blob) => VmValue {
                    r#type: 0,
                    value: blob.clone(),
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
        let meta = ExecutionMeta::new(
            contract.max_gas_amount,
            contract.gas_unit_price,
            AccountAddress::try_from(contract.address).map_err(|err| {
                VMStatus::new(StatusCode::INVALID_DATA)
                    .with_message(format!("Invalid AccountAddress: {:?}", err))
            })?,
        );

        let code = match contract.contract_type {
            0 /*Module*/ => {
                Ok(Code::Module(Module::new(contract.code)))
            }
            1 /*Script*/ => {
                let args = contract.args.into_iter()
                    .map(|arg|
                        match arg.r#type {
                            0 /*Bool*/ => parse_as_bool(&arg.value),
                            1 /*U64*/ => parse_as_u64(&arg.value),
                            2 /*ByteArray*/ => parse_as_byte_array(&arg.value),
                            3 /*Address*/ => parse_as_address(&arg.value),
                            _ => Err(Error::msg("Invalid args type.")),
                        }.map_err(|err| VMStatus::new(StatusCode::INVALID_DATA)
                            .with_message(format!("Invalid contract args [{:?}].", err)))
                    ).collect::<Result<Vec<_>, _>>()?;

                Ok(Code::Script(Script::new(contract.code, args)))
            }
            _ => Err(VMStatus::new(StatusCode::INVALID_DATA)
                .with_message("Invalid contract type.".to_string())),
        }?;

        Ok(Contract { meta, code })
    }
}
