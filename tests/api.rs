use byteorder::{LittleEndian, ByteOrder};

use libra::libra_types;
use libra_types::account_address::AccountAddress;
use libra_types::byte_array::ByteArray;

use dvm_test_kit::*;
use dvm::vm::native::{Reg, dbg};
use dvm::vm::native::oracle::PriceOracle;
use dvm::compiled_protos::vm_grpc::{VmTypeTag, VmArgs};
use lang::banch32::bech32_into_libra;
use data_source::MockDataSource;

#[test]
fn test_create_account() {
    let test_kit = TestKit::new();
    let create_account = "\
        import 0x0.Account;
        main(fresh_address: address) {
          Account.create_account(move(fresh_address));
          return;
        }
    ";
    let bech32_sender_address = "wallets196udj7s83uaw2u4safcrvgyqc0sc3flxuherp6";
    let account_address = AccountAddress::from_hex_literal(&format!(
        "0x{}",
        bech32_into_libra(bech32_sender_address).expect("Invalid bech32 address")
    ))
    .expect("Cannot make AccountAddress");

    let args = vec![VmArgs {
        r#type: VmTypeTag::Address as i32,
        value: bech32_sender_address.to_string(),
    }];
    let res = test_kit.execute_script(create_account, meta(&account_address), args);
    test_kit.assert_success(&res);
    assert!(!res.executions[0].write_set.is_empty());
    test_kit.merge_result(&res);
}

#[test]
fn test_native_func() {
    dbg::PrintByteArray {}.reg_function();

    let test_kit = TestKit::new();

    test_kit.add_std_module(include_str!("./resources/dbg.mvir"));

    let script = "\
        import 0x0.Dbg;
        main(data: bytearray) {
          Dbg.print_byte_array(move(data));
          return;
        }
    ";
    let args = vec![VmArgs {
        r#type: VmTypeTag::ByteArray as i32,
        value: "b\"C001C00D\"".to_string(),
    }];

    let bech32_sender_address = "wallets196udj7s83uaw2u4safcrvgyqc0sc3flxuherp6";
    let account_address = AccountAddress::from_hex_literal(&format!(
        "0x{}",
        bech32_into_libra(bech32_sender_address).unwrap()
    ))
    .unwrap();
    let res = test_kit.execute_script(script, meta(&account_address), args);
    test_kit.assert_success(&res);
}

#[test]
fn test_oracle() {
    let ds = MockDataSource::new();
    let ticker = hex::decode("425443555344").unwrap();

    let mut price = vec![0; 8];
    LittleEndian::write_u64(&mut price, 13);

    ds.insert(
        PriceOracle::make_path(ByteArray::new(ticker)).unwrap(),
        price,
    );
    PriceOracle::new(Box::new(ds)).reg_function();
    let dump = dbg::DumpU64::new();
    dump.clone().reg_function();

    let test_kit = TestKit::new();
    test_kit.add_std_module(include_str!("./resources/dbg.mvir"));

    let script = "\
        import 0x0.Dbg;
        import 0x0.Oracle;

        main() {
          Dbg.dump_u64(Oracle.get_price(h\"425443555344\"));
          return;
        }
    ";

    let bech32_sender_address = "wallets196udj7s83uaw2u4safcrvgyqc0sc3flxuherp6";
    let account_address = AccountAddress::from_hex_literal(&format!(
        "0x{}",
        bech32_into_libra(bech32_sender_address).unwrap()
    ))
    .unwrap();
    let res = test_kit.execute_script(script, meta(&account_address), vec![]);
    test_kit.assert_success(&res);
    assert_eq!(dump.get(), Some(13));
}

#[test]
fn test_publish_module() {
    let test_kit = TestKit::new();
    let bech32_sender_address = "wallets196udj7s83uaw2u4safcrvgyqc0sc3flxuherp6";
    let account_address = AccountAddress::from_hex_literal(&format!(
        "0x{}",
        bech32_into_libra(bech32_sender_address).unwrap()
    ))
    .unwrap();
    let res = test_kit.publish_module(
        include_str!("./resources/module_coin.mvir"),
        meta(&account_address),
    );
    test_kit.assert_success(&res);
    test_kit.merge_result(&res);
}
