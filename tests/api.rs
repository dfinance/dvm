use libra_types::account_address::AccountAddress;

use move_vm_in_cosmos::compiled_protos::vm_grpc::{VmArgs, VmTypeTag};
use move_vm_in_cosmos::test_kit::*;
use move_vm_in_cosmos::vm::{bech32_into_libra_address, Lang};
use move_vm_in_cosmos::vm::native::init_native;

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
    let bech32_sender_address = "cosmos1sxqtxa3m0nh5fu2zkyfvh05tll8fmz8tk2e22e";
    let account_address = AccountAddress::from_hex_literal(&format!(
        "0x{}",
        bech32_into_libra_address(bech32_sender_address).unwrap()
    ))
    .unwrap();
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
    init_native().unwrap();

    let test_kit = TestKit::new(Lang::MvIr);
    let script = "\
        import 0x0.Dbg;
        main(data: bytearray) {
          Dbg.print_byte_array(move(data));
          return;
        }
    ";
    let bech32_sender_address = "cosmos1sxqtxa3m0nh5fu2zkyfvh05tll8fmz8tk2e22e";
    let account_address = AccountAddress::from_hex_literal(&format!(
        "0x{}",
        bech32_into_libra_address(bech32_sender_address).unwrap()
    ))
    .unwrap();
    let args = vec![VmArgs {
        r#type: VmTypeTag::ByteArray as i32,
        value: "b\"C001C00D\"".to_string(),
    }];
    let res = test_kit.execute_script(script, meta(&account_address), args);
    test_kit.assert_success(&res);
}

#[test]
fn test_publish_module() {
    let test_kit = TestKit::new(Lang::MvIr);
    let bech32_sender_address = "cosmos1sxqtxa3m0nh5fu2zkyfvh05tll8fmz8tk2e22e";
    let account_address = AccountAddress::from_hex_literal(&format!(
        "0x{}",
        bech32_into_libra_address(bech32_sender_address).unwrap()
    ))
    .unwrap();
    let res = test_kit.publish_module(
        include_str!("./resources/module_coin.mvir"),
        meta(&account_address),
    );
    test_kit.assert_success(&res);
    test_kit.merge_result(&res);
}
