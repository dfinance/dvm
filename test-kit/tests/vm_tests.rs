use libra::libra_types;
use libra_types::account_address::AccountAddress;
use dvm_test_kit::*;
use runtime::move_vm::{U64Store, AddressStore, VectorU8Store};
use libra::lcs;
use dvm_test_kit::compiled_protos::vm_grpc::{VmArgs, VmTypeTag};

#[test]
fn test_address_as_argument() {
    let test_kit = TestKit::new();
    test_kit.add_std_module(include_str!("resources/store.move"));

    let script = "
        script {
        use 0x0::Store;

        fun main(addr: address) {
            Store::store_address(addr);
        }
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

#[test]
fn test_vector_as_argument() {
    let test_kit = TestKit::new();
    test_kit.add_std_module(include_str!("resources/store.move"));

    let script = "
        script {
        use 0x0::Store;

        fun main(vec: vector<u8>) {
            Store::store_vector_u8(vec);
        }
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
fn test_update_std_module() {
    let test_kit = TestKit::new();
    test_kit.add_std_module(include_str!("resources/store.move"));
    test_kit.add_std_module("module Foo{ public fun foo(): u64 {1}}");

    let load_foo = "\
        script {
        use 0x0::Foo;
        use 0x0::Store;

        fun main() {
            Store::store_u64(Foo::foo());
        }
        }
    ";
    let res = test_kit.execute_script(load_foo, meta(&AccountAddress::random()), vec![]);
    test_kit.assert_success(&res);
    let value: U64Store = lcs::from_bytes(&res.executions[0].write_set[0].value).unwrap();
    assert_eq!(value.val, 1);

    let res = test_kit.publish_module(
        "module Foo{ public fun foo(): u64 {2}}",
        meta(&AccountAddress::default()),
    );
    test_kit.assert_success(&res);
    test_kit.merge_result(&res);
    test_kit.add_std_module(include_str!("resources/store.move"));

    let load_foo = "\
        script {
        use 0x0::Foo;
        use 0x0::Store;

        fun main() {
            Store::store_u64(Foo::foo());
        }
        }
    ";

    let res = test_kit.execute_script(load_foo, meta(&AccountAddress::random()), vec![]);
    test_kit.assert_success(&res);
    let value: U64Store = lcs::from_bytes(&res.executions[0].write_set[0].value).unwrap();
    assert_eq!(value.val, 2);
}
