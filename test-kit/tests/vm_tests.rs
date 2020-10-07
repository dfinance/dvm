use libra::{prelude::*, lcs};
use dvm_test_kit::*;
use runtime::resources::*;
use dvm_net::api::grpc::vm_grpc::{VmArgs, VmStatus, Message, MoveError, vm_status};
use dvm_net::api::grpc::types::VmTypeTag;

#[test]
fn test_sender_as_argument() {
    let test_kit = TestKit::new();
    test_kit.add_std_module(include_str!("resources/store.move"));

    let script = "
        script {
        use 0x1::Store;

        fun main(account: &signer, addr: address) {
            Store::store_address(account, addr);
        }
        }
    ";

    let account_address = AccountAddress::random();
    let args = vec![VmArgs {
        r#type: VmTypeTag::Address as i32,
        value: account_address.to_vec(),
    }];
    let res = test_kit.execute_script(script, gas_meta(), args, vec![], vec![account("0x110")]);
    test_kit.assert_success(&res);
    let value: AddressStore = lcs::from_bytes(&res.write_set[0].value).unwrap();
    assert_eq!(value.val, account_address);
}

#[test]
fn test_senders_as_argument() {
    let test_kit = TestKit::new();
    test_kit.add_std_module(include_str!("resources/store.move"));

    let script = "
        script {
        use 0x1::Store;

        fun main(account: &signer, account_2: &signer, addr: address) {
            Store::store_address(account, addr);
            Store::store_address(account_2, addr);
        }
        }
    ";

    let account_address = AccountAddress::random();
    let args = vec![VmArgs {
        r#type: VmTypeTag::Address as i32,
        value: account_address.to_vec(),
    }];
    let res = test_kit.execute_script(
        script,
        gas_meta(),
        args,
        vec![],
        vec![account("0x110"), account("0x111")],
    );
    test_kit.assert_success(&res);
    let value: AddressStore = lcs::from_bytes(&res.write_set[0].value).unwrap();
    assert_eq!(value.val, account_address);
    let value: AddressStore = lcs::from_bytes(&res.write_set[1].value).unwrap();
    assert_eq!(value.val, account_address);
}

#[test]
fn test_vector_as_argument() {
    let test_kit = TestKit::new();
    test_kit.add_std_module(include_str!("resources/store.move"));

    let script = "
        script {
        use 0x1::Store;

        fun main(account: &signer, vec: vector<u8>) {
            Store::store_vector_u8(account, vec);
        }
        }
    ";

    let vec = AccountAddress::random().to_vec();
    let args = vec![VmArgs {
        r#type: VmTypeTag::Vector as i32,
        value: vec.clone(),
    }];
    let res = test_kit.execute_script(script, gas_meta(), args, vec![], vec![account("0x110")]);
    test_kit.assert_success(&res);
    let value: VectorU8Store = lcs::from_bytes(&res.write_set[0].value).unwrap();
    assert_eq!(value.val, vec);
}

#[test]
fn test_update_std_module() {
    let test_kit = TestKit::new();

    let res = test_kit.publish_module(
        "module Foo{ public fun foo(): u64 {1}}",
        gas_meta(),
        CORE_CODE_ADDRESS,
    );
    test_kit.assert_success(&res);
    test_kit.merge_result(&res);

    test_kit.add_std_module(include_str!("resources/store.move"));

    let load_foo = "\
        script {
        use 0x1::Foo;
        use 0x1::Store;

        fun main(account: &signer) {
            Store::store_u64(account, Foo::foo());
        }
        }
    ";
    let res = test_kit.execute_script(
        load_foo,
        gas_meta(),
        vec![],
        vec![],
        vec![AccountAddress::random()],
    );
    test_kit.assert_success(&res);
    let value: U64Store = lcs::from_bytes(&res.write_set[0].value).unwrap();
    assert_eq!(value.val, 1);

    let res = test_kit.publish_module(
        "module Foo{ public fun foo(): u64 {2}}",
        gas_meta(),
        CORE_CODE_ADDRESS,
    );
    test_kit.assert_success(&res);
    test_kit.merge_result(&res);
    test_kit.add_std_module(include_str!("resources/store.move"));

    let res = test_kit.execute_script(
        load_foo,
        gas_meta(),
        vec![],
        vec![],
        vec![AccountAddress::random()],
    );
    test_kit.assert_success(&res);
    let value: U64Store = lcs::from_bytes(&res.write_set[0].value).unwrap();
    assert_eq!(value.val, 2);
}

#[test]
fn test_update_std_module_1() {
    let test_kit = TestKit::new();

    let res = test_kit.publish_module(
        "module Foo{ public fun foo(): u64 {1}}",
        gas_meta(),
        CORE_CODE_ADDRESS,
    );
    test_kit.assert_success(&res);
    test_kit.merge_result(&res);

    let res = test_kit.publish_module(
        "module Foo{ public fun foo(): u64 {2}}",
        gas_meta(),
        CORE_CODE_ADDRESS,
    );
    test_kit.assert_success(&res);
    test_kit.merge_result(&res);
}

#[test]
fn test_publish_module_data_format_error() {
    let test_kit = TestKit::new();
    let bytecode = test_kit
        .compile(
            "module Foo{ public fun foo(): u64 {1}}",
            Some(CORE_CODE_ADDRESS),
        )
        .unwrap();

    let resp =
        test_kit.publish_module_raw(bytecode, u64::MAX, u64::MAX, CORE_CODE_ADDRESS.to_vec());

    assert_eq!(
        resp.status,
        Some(VmStatus {
            message: Some(Message {
                text: "max_gas_amount value must be in the range from 0 to 18446744073709551"
                    .to_owned()
            }),
            error: Some(vm_status::Error::MoveError(MoveError {
                status_code: StatusCode::DATA_FORMAT_ERROR as u64
            }))
        })
    )
}

#[test]
fn test_execute_script_data_format_error() {
    let test_kit = TestKit::new();
    let resp = test_kit.execute_script(
        "script {
        fun main(_account: &signer, _int: u64) {}
        }",
        gas_meta(),
        vec![VmArgs {
            r#type: VmTypeTag::U64 as i32,
            value: vec![0x0, 0x1, 0x2, 0x3],
        }],
        vec![],
        vec![AccountAddress::random()],
    );

    assert_eq!(
        resp.status,
        Some(VmStatus {
            message: Some(Message {
                text: "Invalid u64 argument length. Expected 8 byte.".to_owned()
            }),
            error: Some(vm_status::Error::MoveError(MoveError {
                status_code: StatusCode::DATA_FORMAT_ERROR as u64
            }))
        })
    )
}
