use move_vm_in_cosmos::test_kit::*;
use libra_types::account_address::AccountAddress;
use move_vm_in_cosmos::vm::Lang;
use move_vm_in_cosmos::vm::native::{dbg, Reg};
use move_vm_in_cosmos::ds::MockDataSource;
use move_vm_in_cosmos::vm::native::oracle::PriceOracle;
use byteorder::{LittleEndian, ByteOrder};
use libra_types::byte_array::ByteArray;

#[test]
fn test_create_account() {
    let test_kit = TestKit::new(Lang::MvIr);
    let acc_1 = AccountAddress::random();
    let create_account = "\
        import 0x0.LibraAccount;
        main(fresh_address: address, initial_amount: u64) {
          LibraAccount.create_new_account(move(fresh_address), move(initial_amount));
          return;
        }
    ";
    let res = test_kit.execute_script(create_account, meta(&acc_1), &[&addr(&acc_1), "1000"]);
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

    let acc_1 = AccountAddress::random();
    let script = "\
        import 0x0.Dbg;
        main(data: bytearray) {
          Dbg.print_byte_array(move(data));
          return;
        }
    ";
    let res = test_kit.execute_script(script, meta(&acc_1), &["b\"C001C00D\""]);
    test_kit.assert_success(&res);
}

#[test]
fn test_oracle() {
    let ds = MockDataSource::without_std();
    let ticker = hex::decode("425443555344").unwrap();

    let mut price = vec![0; 16];
    LittleEndian::write_u128(&mut price, 13);

    ds.insert(
        PriceOracle::make_path(ByteArray::new(ticker)).unwrap(),
        price,
    );
    PriceOracle::new(Box::new(ds)).reg_function();
    let dump = dbg::DumpU128::new();
    dump.clone().reg_function();

    let test_kit = TestKit::new(Lang::MvIr);

    let res = test_kit.publish_module(
        include_str!("./resources/dbg.mvir"),
        meta(&AccountAddress::default()),
    );
    test_kit.assert_success(&res);
    test_kit.merge_result(&res);

    let acc_1 = AccountAddress::random();
    let script = "\
        import 0x0.Dbg;
        import 0x0.Oracle;

        main() {
          Dbg.dump_u128(Oracle.get_price(h\"425443555344\"));
          return;
        }
    ";
    let res = test_kit.execute_script(script, meta(&acc_1), &[]);
    test_kit.assert_success(&res);
    assert_eq!(dump.get(), Some(13));
}

#[test]
fn test_publish_module() {
    let test_kit = TestKit::new(Lang::MvIr);
    let acc_1 = AccountAddress::random();
    let res = test_kit.publish_module(include_str!("./resources/module_coin.mvir"), meta(&acc_1));
    test_kit.assert_success(&res);
    test_kit.merge_result(&res);
}
