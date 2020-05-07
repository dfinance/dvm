use libra::libra_types::{account_address::AccountAddress, language_storage::ModuleId};
use libra::move_core_types::identifier::Identifier;
use ds::MockDataSource;
use dvm_lang::stdlib::zero_sdt;
use std::collections::HashSet;
use libra::libra_vm::{
    file_format::{CompiledScript, CompiledModule},
    access::ModuleAccess,
};
use dvm_lang::compiler::{
    make_address, compile,
    preprocessor::{replace_u_literal, str_xxhash},
    meta::ModuleMeta,
    Compiler,
};

#[test]
fn test_u_literal() {
    assert_eq!(
        replace_u_literal("Oracle.get_price(#\"USD\") + Oracle.get_price(#\"BTC\") = #\"USDBTC\"",),
        format!(
            "Oracle.get_price({}) + Oracle.get_price({}) = {}",
            str_xxhash("usd"),
            str_xxhash("btc"),
            str_xxhash("usdbtc")
        )
    )
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
    compiler.compile(module, &AccountAddress::random()).unwrap();
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
    let program = "fun main() {}";
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
        fun main() {\
            0x1::M::foo();
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
            use wallet1me0cdn52672y7feddy7tgcj6j4dkzq2su745vh::Account;
            fun main() {
               Account::foo();
            }
        ";

    let script = compile(
        program,
        vec![(
            dep,
            &make_address("0xde5f86ce8ad7944f272d693cb4625a955b61015000000000"),
        )],
        &AccountAddress::default(),
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
        "de5f86ce8ad7944f272d693cb4625a955b61015000000000"
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
            &make_address("0xde5f86ce8ad7944f272d693cb4625a955b61015000000000"),
        )],
        &AccountAddress::default(),
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
        "de5f86ce8ad7944f272d693cb4625a955b61015000000000"
    );
}

#[test]
fn test_create_compiler() {
    let view = MockDataSource::new();
    let _compiler = Compiler::new(view);
}

#[test]
fn test_move_meta() {
    let view = MockDataSource::new();
    let compiler = Compiler::new(view);
    let meta = compiler
        .code_meta(&include_str!("resources/transaction_fee_distribution.move"))
        .unwrap();
    assert_eq!(&meta.module_name, "TransactionFeeDistribution");
    assert_eq!(
        meta.dep_list.into_iter().collect::<HashSet<_>>(),
        vec![
            ModuleId::new(
                AccountAddress::default(),
                Identifier::new("ValidatorSet").unwrap(),
            ),
            ModuleId::new(
                AccountAddress::default(),
                Identifier::new("Account").unwrap(),
            ),
            ModuleId::new(AccountAddress::default(), Identifier::new("Coin").unwrap()),
            ModuleId::new(
                AccountAddress::default(),
                Identifier::new("Transaction").unwrap(),
            ),
        ]
        .into_iter()
        .collect::<HashSet<_>>()
    );
}

#[test]
fn test_script_meta() {
    let view = MockDataSource::new();
    let compiler = Compiler::new(view);
    let meta = compiler
        .code_meta(
            "
                use 0x0::Oracle;
                fun main(payee: address, amount: u64) {
                    Oracle::get_price(#\"\");
                }
            ",
        )
        .unwrap();
    assert_eq!(
        meta,
        ModuleMeta {
            module_name: "main".to_string(),
            dep_list: vec![ModuleId::new(
                AccountAddress::default(),
                Identifier::new("Oracle").unwrap(),
            ),],
        }
    )
}

#[test]
fn test_move_script_meta() {
    let view = MockDataSource::new();
    let compiler = Compiler::new(view);
    let meta = compiler
        .code_meta(
            "\
            use 0x0::Coins;

            fun main(payee: address, amount: u64) {
                0x0::Account::mint_to_address(payee, amount)
            }
            ",
        )
        .unwrap();
    assert_eq!(&meta.module_name, "main");
    assert_eq!(
        meta.dep_list.into_iter().collect::<HashSet<_>>(),
        vec![
            ModuleId::new(AccountAddress::default(), Identifier::new("Coins").unwrap()),
            ModuleId::new(
                AccountAddress::default(),
                Identifier::new("Account").unwrap(),
            ),
        ]
        .into_iter()
        .collect::<HashSet<_>>()
    );
}

#[test]
fn test_build_move() {
    let compiler = Compiler::new(MockDataSource::with_write_set(zero_sdt()));

    compiler
        .compile(
            "\
            fun main() {
            }
            ",
            &AccountAddress::default(),
        )
        .unwrap();
}
