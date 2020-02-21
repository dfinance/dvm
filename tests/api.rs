use byteorder::{LittleEndian, ByteOrder};
use move_vm_in_cosmos::test_kit::*;
use libra_types::account_address::AccountAddress;
use move_vm_in_cosmos::vm::{Lang, bech32_utils};
use move_vm_in_cosmos::vm::native::{Reg, dbg};
use move_vm_in_cosmos::vm::native::oracle::PriceOracle;
use move_vm_in_cosmos::ds::MockDataSource;
use move_vm_in_cosmos::compiled_protos::vm_grpc::{VmTypeTag, VmArgs};
use libra_types::byte_array::ByteArray;

#[test]
fn test_create_account() {
    let test_kit = TestKit::new(Lang::MvIr);
    let create_account = "\
        import 0x0.LibraAccount;
        main(fresh_address: address, initial_amount: u64) {
          LibraAccount.create_new_account(move(fresh_address), move(initial_amount));
          return;
        }
    ";
    let bech32_sender_address = "wallets196udj7s83uaw2u4safcrvgyqc0sc3flxuherp6";
    let account_address = AccountAddress::from_hex_literal(&format!(
        "0x{}",
        bech32_utils::bech32_into_libra(bech32_sender_address).expect("Invalid bech32 address")
    ))
    .expect("Cannot make AccountAddress");

    let args = vec![
        VmArgs {
            r#type: VmTypeTag::Address as i32,
            value: bech32_sender_address.to_string(),
        },
        VmArgs {
            r#type: VmTypeTag::U64 as i32,
            value: "1000".to_string(),
        },
    ];
    let res = test_kit.execute_script(create_account, meta(&account_address), args);
    test_kit.assert_success(&res);
    test_kit.merge_result(&res);
}

#[test]
fn test_native_func() {
    dbg::PrintByteArray {}.reg_function();

    let test_kit = TestKit::new(Lang::MvIr);

    let res = test_kit.publish_module(
        include_str!("./resources/dbg.mvir"),
        meta(&AccountAddress::default()),
    );
    test_kit.assert_success(&res);
    test_kit.merge_result(&res);

    let bech32_sender_address = "wallets196udj7s83uaw2u4safcrvgyqc0sc3flxuherp6";
    let account_address = AccountAddress::from_hex_literal(&format!(
        "0x{}",
        bech32_utils::bech32_into_libra(bech32_sender_address).unwrap()
    ))
    .unwrap();
    let res = test_kit.publish_module(include_str!("./resources/dbg.mvir"), meta(&account_address));
    test_kit.assert_success(&res);
    test_kit.merge_result(&res);

    let script = "\
        import wallets196udj7s83uaw2u4safcrvgyqc0sc3flxuherp6.Dbg;
        main(data: bytearray) {
          Dbg.print_byte_array(move(data));
          return;
        }
    ";
    let args = vec![VmArgs {
        r#type: VmTypeTag::ByteArray as i32,
        value: "b\"C001C00D\"".to_string(),
    }];
    let res = test_kit.execute_script(script, meta(&account_address), args);
    test_kit.assert_success(&res);
}

#[test]
fn test_oracle() {
    let ds = MockDataSource::without_std();
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

    let test_kit = TestKit::new(Lang::MvIr);

    let bech32_sender_address = "wallets196udj7s83uaw2u4safcrvgyqc0sc3flxuherp6";
    let account_address = AccountAddress::from_hex_literal(&format!(
        "0x{}",
        bech32_utils::bech32_into_libra(bech32_sender_address).unwrap()
    ))
    .unwrap();

    let res = test_kit.publish_module(include_str!("./resources/dbg.mvir"), meta(&account_address));
    test_kit.assert_success(&res);
    test_kit.merge_result(&res);

    let script = "\
        import wallets196udj7s83uaw2u4safcrvgyqc0sc3flxuherp6.Dbg;
        import wallets196udj7s83uaw2u4safcrvgyqc0sc3flxuherp6.Oracle;

        main() {
          Dbg.dump_u64(Oracle.get_price(h\"425443555344\"));
          return;
        }
    ";
    let res = test_kit.execute_script(script, meta(&account_address), vec![]);
    test_kit.assert_success(&res);
    assert_eq!(dump.get(), Some(13));
}

#[test]
fn test_publish_module() {
    let test_kit = TestKit::new(Lang::MvIr);
    let bech32_sender_address = "wallets196udj7s83uaw2u4safcrvgyqc0sc3flxuherp6";
    let account_address = AccountAddress::from_hex_literal(&format!(
        "0x{}",
        bech32_utils::bech32_into_libra(bech32_sender_address).unwrap()
    ))
    .unwrap();
    let res = test_kit.publish_module(
        include_str!("./resources/module_coin.mvir"),
        meta(&account_address),
    );
    test_kit.assert_success(&res);
    test_kit.merge_result(&res);
}
