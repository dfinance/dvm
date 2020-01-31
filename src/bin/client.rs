use move_vm_in_cosmos::grpc;
use http::Uri;
use structopt::StructOpt;
use move_vm_in_cosmos::compiled_protos::vm_grpc::VmExecuteRequest;
use move_vm_in_cosmos::compiled_protos::vm_grpc::vm_service_client::VmServiceClient;

#[derive(Debug, StructOpt)]
struct Options {
    #[structopt(name = "server_address", help = "Server internet address")]
    server_address: Uri,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = Options::from_args();

    let mut client = VmServiceClient::connect(options.server_address).await?;
    //  req: execute_contracts
    {
        for i in 0u8..3u8 {
            println!("sending empty requests");
            let request = tonic::Request::new(VmExecuteRequest {
                contracts: Vec::default(),
                options: Default::default(), // u64
            });
            let response = client.execute_contracts(request).await?;
            println!("RESPONSE:\n{:?}", response);
        }
        for i in 0u8..3u8 {
            println!("sending real requests");
            let exec_req = my_tests::test_publish_mod()?;
            println!("{} > MOD REQUEST: {:?}", i, exec_req);
            let response = client.execute_contracts(exec_req).await?;
            println!("{} < MOD RESPONSE: {:?}", i, response);
        }
    }
    Ok(())
}

mod my_tests {
    // use move_vm_in_cosmos::move_lang::*;
    // use move_vm_in_cosmos::libra_types::*;

    use super::*;
    use libra_types::account_address::AccountAddress;
    use move_vm_in_cosmos::test_kit::Lang;
    use move_vm_in_cosmos::compiled_protos::vm_grpc::VmContract;

    pub fn test_publish_mod() -> Result<VmExecuteRequest, Box<dyn std::error::Error>> {
        let compiler = Lang::MvIr.compiler();
        let sender = AccountAddress::random();
        let source = include_str!("../../tests/resources/module_coin.mvir");
        let module = compiler.build_module(source, &sender);
        Ok(VmExecuteRequest {
            contracts: vec![VmContract {
                address: sender.to_vec(),
                max_gas_amount: 0,
                gas_unit_price: 0,
                code: module,
                contract_type: 0, // Module
                args: vec![],
            }],
            options: 0,
        })
    }
}