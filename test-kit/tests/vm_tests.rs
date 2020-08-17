use libra::{prelude::*, lcs};
use dvm_test_kit::*;
use dvm_test_kit::compiled_protos::vm_grpc::{VmArgs, VmTypeTag};
use runtime::resources::*;

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
