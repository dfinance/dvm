use dvm_test_kit::TestKit;
use byteorder::{LittleEndian, ByteOrder};
use libra::libra_types;
use libra_types::account_address::AccountAddress;
use dvm_test_kit::*;
use libra::move_vm_natives::oracle;
use compiler::preprocessor::str_xxhash;
use runtime::move_vm::{U64Store, AddressStore};
use libra::lcs;
use dvm_test_kit::compiled_protos::vm_grpc::{VmArgs, VmTypeTag};

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

    let script = "
        script {
        use 0x0::Store;
        use 0x0::Oracle;

        fun main() {
            Store::store_u64(Oracle::get_price(#\"USDBTC\"));
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
        use 0x0::Oracle;

        fun main() {
          Store::store_u64(Oracle::get_price(#\"USDxrp\"));
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

#[test]
fn test_native_save_balance() {
    let test_kit = TestKit::new();
    test_kit.add_std_module(include_str!("resources/transaction.move"));
    test_kit.add_std_module(include_str!("resources/store.move"));
    test_kit.add_std_module(include_str!("resources/event.move"));
    test_kit.add_std_module(include_str!("resources/account.move"));

    let sender = AccountAddress::random();
    let recipient = AccountAddress::random();

    let send_script = "\
        script {
        use 0x0::Account;

        fun main(coin_1_balance: u64, coin_2_balance: u64, addr: address) {
            Account::save_coin<Account::Coin1>(coin_1_balance, addr);
            Account::save_coin<Account::Coin2>(coin_2_balance, addr);
        }
        }
    ";

    let coin_1 = 13;
    let coin_2 = 90;

    let args = vec![
        VmArgs {
            r#type: VmTypeTag::U64 as i32,
            value: coin_1.to_string(),
        },
        VmArgs {
            r#type: VmTypeTag::U64 as i32,
            value: coin_2.to_string(),
        },
        VmArgs {
            r#type: VmTypeTag::Address as i32,
            value: format!("0x{}", recipient),
        },
    ];
    let res = test_kit.execute_script(send_script, meta(&sender), args);
    test_kit.assert_success(&res);
    test_kit.merge_result(&res);

    let recipient_coin_1_script = "\
        script {
        use 0x0::Account;
        use 0x0::Store;

        fun main() {
            Store::store_u64(Account::balance<Account::Coin1>());
        }
        }
    ";
    let res = test_kit.execute_script(recipient_coin_1_script, meta(&recipient), vec![]);
    test_kit.assert_success(&res);
    let value: U64Store = lcs::from_bytes(&res.executions[0].write_set[0].value).unwrap();
    assert_eq!(coin_1, value.val);

    let recipient_coin_2_script = "\
        script {
        use 0x0::Account;
        use 0x0::Store;

        fun main() {
            Store::store_u64(Account::balance<Account::Coin2>());
        }
        }
    ";
    let res = test_kit.execute_script(recipient_coin_2_script, meta(&recipient), vec![]);
    test_kit.assert_success(&res);
    let value: U64Store = lcs::from_bytes(&res.executions[0].write_set[0].value).unwrap();
    assert_eq!(coin_2, value.val);
}

#[test]
fn test_native_save_account() {
    let test_kit = TestKit::empty();
    test_kit.add_std_module(include_str!("resources/transaction.move"));
    test_kit.add_std_module(include_str!("resources/event.move"));
    test_kit.add_std_module(include_str!("resources/account.move"));
    test_kit.add_std_module(include_str!("resources/store.move"));

    let create_account_script = "\
        script {
        use 0x0::Account;

        fun main(t_value: u64, addr: address) {
            Account::create_account(t_value, addr);
        }
        }
    ";

    let account = AccountAddress::random();

    let t_value = 13;
    let args = vec![
        VmArgs {
            r#type: VmTypeTag::U64 as i32,
            value: t_value.to_string(),
        },
        VmArgs {
            r#type: VmTypeTag::Address as i32,
            value: format!("0x{}", account),
        },
    ];
    let res = test_kit.execute_script(create_account_script, meta(&account), args);
    test_kit.assert_success(&res);
    test_kit.merge_result(&res);

    let load_t = "\
        script {
        use 0x0::Account;
        use 0x0::Store;

        fun main() {
            Store::store_u64(Account::get_t_value());
        }
        }
    ";
    let res = test_kit.execute_script(load_t, meta(&account), vec![]);
    test_kit.assert_success(&res);
    let value: U64Store = lcs::from_bytes(&res.executions[0].write_set[0].value).unwrap();
    assert_eq!(t_value, value.val);
}

#[test]
fn test_register_token_info() {
    let test_kit = TestKit::empty();
    test_kit.add_std_module(include_str!("resources/dfinance.move"));

    let script = "\
        script {
        use 0x0::Dfinance;

        fun main(t_value: u64) {
            Dfinance::store_info<Dfinance::SimpleCoin>(t_value);
        }
        }
    ";

    let account = AccountAddress::random();

    let t_value = 13;
    let args = vec![VmArgs {
        r#type: VmTypeTag::U64 as i32,
        value: t_value.to_string(),
    }];
    let res = test_kit.execute_script(script, meta(&account), args);
    test_kit.assert_success(&res);
    let value: U64Store = lcs::from_bytes(&res.executions[0].write_set[0].value).unwrap();
    assert_eq!(t_value, value.val);
}
