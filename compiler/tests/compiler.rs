use ds::MockDataSource;
use libra::prelude::*;
use dvm_compiler::Compiler;
use anyhow::Error;

pub fn compile(
    source: &str,
    dep_list: Vec<(&str, &AccountAddress)>,
    address: &AccountAddress,
) -> Result<Vec<u8>, Error> {
    let ds = MockDataSource::new();
    let compiler = Compiler::new(ds.clone());
    for (code, address) in dep_list {
        ds.publish_module(compiler.compile(code, Some(*address))?)?;
    }

    compiler.compile(source, Some(*address))
}

pub fn compile_script(
    source: &str,
    dep: Vec<(&str, &AccountAddress)>,
    address: &AccountAddress,
) -> CompiledScript {
    CompiledScript::deserialize(&compile(source, dep, address).unwrap()).unwrap()
}

pub fn make_address(address: &str) -> AccountAddress {
    AccountAddress::from_hex_literal(address).unwrap()
}

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
    compiler
        .compile(module, Some(AccountAddress::random()))
        .unwrap();
}

#[test]
pub fn test_build_module_success() {
    let program = "module M {}";
    compile(program, vec![], &AccountAddress::random()).unwrap();
}

#[test]
pub fn test_build_module_failed() {
    let program = "module M {";
    let error = compile(program, vec![], &AccountAddress::random())
        .err()
        .unwrap();
    assert!(error.to_string().contains("Unexpected end-of-file"));
}

#[test]
pub fn test_build_script() {
    let program = "script{fun main() {}}";
    compile(program, vec![], &AccountAddress::random()).unwrap();
}

#[test]
pub fn test_build_script_with_dependence() {
    let dep = "\
        module M {
            public fun foo(): u64 {
                1
            }
        }
        ";
    let program = "\
        script {
        fun main() {\
            0x1::M::foo();
        }
        }";

    compile(
        program,
        vec![(dep, &make_address("0x1"))],
        &AccountAddress::random(),
    )
    .unwrap();
}

#[test]
fn test_parse_script_with_bech32_addresses() {
    let dep = r"
            module Account {
                public fun foo() {}
            }
        ";

    let program = "
            script {
            use wallet1me0cdn52672y7feddy7tgcj6j4dkzq2su745vh::Account;

            fun main() {
               Account::foo();
            }
            }
        ";

    let script = compile(
        program,
        vec![(
            dep,
            &make_address("0xde5f86ce8ad7944f272d693cb4625a955b610150"),
        )],
        &CORE_CODE_ADDRESS,
    )
    .unwrap();

    let script = CompiledScript::deserialize(&script)
        .unwrap()
        .into_module()
        .1;
    let module = script
        .module_handles()
        .iter()
        .find(|h| script.identifier_at(h.name).to_string() == "Account")
        .unwrap();
    let address = script.address_identifier_at(module.address);
    assert_eq!(
        address.to_string(),
        "DE5F86CE8AD7944F272D693CB4625A955B610150"
    );
}

#[test]
fn test_parse_module_with_bech32_addresses() {
    let dep = r"
            module Account {
                public fun foo() {}
            }
        ";

    let program = "
            module M {
                use wallet1me0cdn52672y7feddy7tgcj6j4dkzq2su745vh::Account;
                fun foo() {
                    Account::foo();
                }
            }
        ";

    let main_module = compile(
        program,
        vec![(
            dep,
            &make_address("0xde5f86ce8ad7944f272d693cb4625a955b610150"),
        )],
        &CORE_CODE_ADDRESS,
    )
    .unwrap();
    let main_module = CompiledModule::deserialize(&main_module).unwrap();

    let module = main_module
        .module_handles()
        .iter()
        .find(|h| main_module.identifier_at(h.name).to_string() == "Account")
        .unwrap();
    let address = main_module.address_identifier_at(module.address);
    assert_eq!(
        address.to_string(),
        "DE5F86CE8AD7944F272D693CB4625A955B610150"
    );
}

#[test]
fn test_create_compiler() {
    let view = MockDataSource::new();
    let _compiler = Compiler::new(view);
}

#[test]
fn test_build_move() {
    let compiler = Compiler::new(MockDataSource::new());

    compiler
        .compile(
            "\
            script {
                fun main() {}
            }
            ",
            Some(CORE_CODE_ADDRESS),
        )
        .unwrap();
}
