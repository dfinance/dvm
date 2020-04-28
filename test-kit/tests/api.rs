use byteorder::{LittleEndian, ByteOrder};
use libra::libra_types;
use libra_types::account_address::AccountAddress;
use libra::move_vm_types::native_functions::oracle;
use dvm_test_kit::*;
use lang::{banch32::bech32_into_libra, compiler::str_xxhash};
use runtime::move_vm::{U64Store, AddressStore};
use libra::lcs;

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

    let account_address = account("df1pfk58n7j62uenmam7f9ncu6qnffc2q5dpwuute");

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

    let account_address = account("df1pfk58n7j62uenmam7f9ncu6qnffc2q5dpwuute");

    let res = test_kit.execute_script(script, meta(&account_address), vec![]);
    test_kit.assert_success(&res);
    let value: AddressStore = lcs::from_bytes(&res.executions[0].write_set[0].value).unwrap();
    assert_eq!(value.val, account_address);
}

#[test]
fn test_kit_pipline() {
    let test_kit = TestKit::new();
    test_oracle(&test_kit);
    test_native_function(&test_kit);
}

fn account(bech32: &str) -> AccountAddress {
    AccountAddress::from_hex_literal(&format!("0x{}", bech32_into_libra(bech32).unwrap())).unwrap()
}
