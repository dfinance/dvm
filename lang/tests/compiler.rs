use dvm_lang::compiler::Compiler;
use libra::libra_types::account_address::AccountAddress;
use ds::MockDataSource;

#[test]
fn test_hex_lateral() {
    let compiler = Compiler::new(MockDataSource::default());
    let module = "
        module Test {
            fun test_hex() {
                let _hex = x\"ff8000\";
            }
        }
    ";
    compiler.compile(module, &AccountAddress::random()).unwrap();
}
