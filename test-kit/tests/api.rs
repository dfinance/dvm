use byteorder::{LittleEndian, ByteOrder};
use libra::libra_types;
use libra_types::account_address::AccountAddress;
use libra::move_vm_types::native_functions::oracle;
use dvm_test_kit::*;
use lang::compiler::preprocessor::str_xxhash;
use runtime::move_vm::{U64Store, AddressStore, VectorU8Store};
use libra::lcs;
use dvm_test_kit::compiled_protos::vm_grpc::{VmArgs, VmTypeTag};

fn test_oracle(test_kit: &TestKit) {
    let price = 13;
    let mut price_buff = vec![0; 8];
    LittleEndian::write_u64(&mut price_buff, price);
    test_kit
        .data_source()
        .insert(oracle::make_path(str_xxhash("usdbtc")).unwrap(), price_buff);

    test_kit.add_std_module(include_str!("resources/store.move"));

    let script = "
        use 0x0::Store;
        use 0x0::Oracle;

        fun main() {
            Store::store_u64(Oracle::get_price(#\"USDBTC\"));
        }
    ";

    let account_address = account("0x110");

    let res = test_kit.execute_script(script, meta(&account_address), vec![]);
    test_kit.assert_success(&res);
    let value: U64Store = lcs::from_bytes(&res.executions[0].write_set[0].value).unwrap();
    assert_eq!(price, value.val);

    let script = "
        use 0x0::Store;
        use 0x0::Oracle;

        fun main() {
          Store::store_u64(Oracle::get_price(#\"USDxrp\"));
        }
    ";
    let res = test_kit.execute_script(script, meta(&account_address), vec![]);
    assert_eq!(
        "Price is not found",
        res.executions[0].status_struct.as_ref().unwrap().message
    );
}

fn test_native_function(test_kit: &TestKit) {
    test_kit.add_std_module(include_str!("resources/store.move"));

    let script = "
        use 0x0::Store;
        use 0x0::Transaction;

        fun main() {
            Store::store_address(Transaction::sender());
        }
    ";

    let account_address = account("0x110");

    let res = test_kit.execute_script(script, meta(&account_address), vec![]);
    test_kit.assert_success(&res);
    let value: AddressStore = lcs::from_bytes(&res.executions[0].write_set[0].value).unwrap();
    assert_eq!(value.val, account_address);
}

fn test_address_as_argument(test_kit: &TestKit) {
    let script = "
        use 0x0::Store;

        fun main(addr: address) {
            Store::store_address(addr);
        }
    ";

    let account_address = AccountAddress::random();
    let args = vec![VmArgs {
        r#type: VmTypeTag::Address as i32,
        value: format!("0x{}", account_address),
    }];
    let res = test_kit.execute_script(script, meta(&account("0x110")), args);
    test_kit.assert_success(&res);
    let value: AddressStore = lcs::from_bytes(&res.executions[0].write_set[0].value).unwrap();
    assert_eq!(value.val, account_address);
}

fn test_vector_as_argument(test_kit: &TestKit) {
    let script = "
        use 0x0::Store;

        fun main(vec: vector<u8>) {
            Store::store_vector_u8(vec);
        }
    ";

    let vec = AccountAddress::random().to_vec();
    let args = vec![VmArgs {
        r#type: VmTypeTag::ByteArray as i32,
        value: format!("x\"{}\"", hex::encode(vec.clone())),
    }];
    let res = test_kit.execute_script(script, meta(&account("0x110")), args);
    test_kit.assert_success(&res);
    let value: VectorU8Store = lcs::from_bytes(&res.executions[0].write_set[0].value).unwrap();
    assert_eq!(value.val, vec);
}

#[test]
fn test_kit_pipeline() {
    let test_kit = TestKit::new();
    test_oracle(&test_kit);
    test_native_function(&test_kit);
    test_address_as_argument(&test_kit);
    test_vector_as_argument(&test_kit);
}

fn account(addr: &str) -> AccountAddress {
    AccountAddress::from_hex_literal(addr).unwrap()
}
