//! Server implementation on tonic & tokio.

use std::fs;

use http::Uri;
use libra_types::account_address::AccountAddress;
use structopt::StructOpt;

use move_vm_in_cosmos::compiled_protos::vm_grpc::{VmExecuteRequest, VmArgs, VmContract};
use move_vm_in_cosmos::compiled_protos::vm_grpc::vm_service_client::VmServiceClient;
use move_vm_in_cosmos::vm::Lang;

#[derive(Debug, StructOpt)]
struct Options {
    #[structopt(name = "server_address", help = "Server internet address")]
    server_address: Uri,
}

fn get_execute_script_request() -> Result<VmExecuteRequest, Box<dyn std::error::Error>> {
    let compiler = Lang::MvIr.compiler();
    let sender = AccountAddress::random();
    let source = fs::read_to_string("tests/resources/script.mvir").unwrap();
    let binary_script = compiler.build_script(&source, &sender, false).unwrap();
    let amount_to_withdraw = VmArgs {
        r#type: 1,
        value: "100".to_string(),
    };
    let address = VmArgs {
        r#type: 3,
        value: format!("0x{}", sender.to_string()),
    };
    let vm_contract = VmContract {
        address: sender.to_string(),
        max_gas_amount: 100_000,
        gas_unit_price: 1,
        code: binary_script,
        contract_type: 1, // Script
        args: vec![amount_to_withdraw, address],
    };

    Ok(VmExecuteRequest {
        contracts: vec![vm_contract],
        options: 0,
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = Options::from_args();

    let exec_req = get_execute_script_request()?;
    let mut client = VmServiceClient::connect(options.server_address).await?;
    let response = client.execute_contracts(exec_req).await?;
    dbg!(response);
    Ok(())
}
