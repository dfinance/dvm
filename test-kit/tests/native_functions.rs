use dvm_test_kit::TestKit;
use byteorder::{LittleEndian, ByteOrder};
use dvm_test_kit::*;
use libra::move_vm_natives::oracle;
use runtime::move_vm::{U64Store, AddressStore};
use libra::lcs;
use twox_hash::XxHash64;
use std::hash::Hasher;

fn str_xxhash(ticker: &str) -> u64 {
    let mut hash = XxHash64::default();
    Hasher::write(&mut hash, ticker.as_bytes());
    Hasher::finish(&hash)
}

#[test]
fn test_oracle() {
    let test_kit = TestKit::new();
    let price = 13;
    let mut price_buff = vec![0; 8];
    LittleEndian::write_u64(&mut price_buff, price);
    test_kit
        .data_source()
        .insert(oracle::make_path(str_xxhash("usdbtc")).unwrap(), price_buff);

    test_kit.add_std_module(include_str!("resources/store.move"));
    test_kit.add_std_module(include_str!("resources/currency.move"));

    let script = "
        script {
        use 0x0::Store;
        use 0x0::Currency;
        use 0x0::Oracle;

        fun main() {
            Store::store_u64(Oracle::get_price<Currency::USD, Currency::BTC>());
        }
        }
    ";

    let account_address = account("0x110");

    let res = test_kit.execute_script(script, meta(&account_address), vec![]);
    test_kit.assert_success(&res);
    let value: U64Store = lcs::from_bytes(&res.executions[0].write_set[0].value).unwrap();
    assert_eq!(price, value.val);

    let script = "
        script {
        use 0x0::Store;
        use 0x0::Currency;
        use 0x0::Oracle;

        fun main() {
          Store::store_u64(Oracle::get_price<Currency::USD, Currency::ETH>());
        }
        }
    ";
    let res = test_kit.execute_script(script, meta(&account_address), vec![]);
    assert_eq!(
        "Price is not found",
        res.executions[0].status_struct.as_ref().unwrap().message
    );
}

#[test]
fn test_native_function() {
    let test_kit = TestKit::new();
    test_kit.add_std_module(include_str!("resources/store.move"));

    let script = "
        script {
        use 0x0::Store;
        use 0x0::Transaction;

        fun main() {
            Store::store_address(Transaction::sender());
        }
        }
    ";

    let account_address = account("0x110");

    let res = test_kit.execute_script(script, meta(&account_address), vec![]);
    test_kit.assert_success(&res);
    let value: AddressStore = lcs::from_bytes(&res.executions[0].write_set[0].value).unwrap();
    assert_eq!(value.val, account_address);
}
