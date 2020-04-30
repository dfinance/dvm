mod module_loader;
mod mv;

pub use mv::Move;
mod imports;
mod name_pull;

use libra::libra_state_view::StateView;
use libra::libra_types::account_address::AccountAddress;
use anyhow::Error;
use libra::libra_types::language_storage::ModuleId;
use crate::compiler::module_loader::ModuleLoader;
use crate::pattern;
use twox_hash::XxHash64;
use std::hash::Hasher;

#[derive(Clone)]
pub struct Compiler<S>
where
    S: StateView + Clone,
{
    mv: Move<S>,
}

impl<S> Compiler<S>
where
    S: StateView + Clone,
{
    pub fn new(view: S) -> Compiler<S> {
        let loader = ModuleLoader::new(view);
        Compiler {
            mv: Move::new(loader),
        }
    }

    pub fn compile(&self, code: &str, address: &AccountAddress) -> Result<Vec<u8>, Error> {
        if self.may_be_script(code) {
            self.mv.build_script(code, address)
        } else {
            self.mv.build_module(code, address)
        }
    }

    pub fn code_meta(&self, code: &str) -> Result<ModuleMeta, Error> {
        self.mv.module_meta(code)
    }

    fn may_be_script(&self, code: &str) -> bool {
        code.contains("main") && !code.contains("module")
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
pub struct ModuleMeta {
    pub module_name: String,
    pub dep_list: Vec<ModuleId>,
}

pub trait Builder {
    fn build_module(&self, code: &str, address: &AccountAddress) -> Result<Vec<u8>, Error>;

    fn build_script(&self, code: &str, address: &AccountAddress) -> Result<Vec<u8>, Error>;

    fn module_meta(&self, code: &str) -> Result<ModuleMeta, Error>;
}

pub fn str_xxhash(val: &str) -> u64 {
    let mut hash = XxHash64::default();
    Hasher::write(&mut hash, val.as_bytes());
    Hasher::finish(&hash)
}

pub fn replace_u_literal(code: &str) -> String {
    let mut replaced = code.to_string();
    let regex = pattern!(r#"#".*?""#);

    let replace_list = regex
        .find_iter(code)
        .map(|mat| {
            let content = mat
                .as_str()
                .to_lowercase()
                .chars()
                .skip(2)
                .take(mat.as_str().len() - 3)
                .collect::<String>();

            (mat.range(), format!("{}", str_xxhash(&content)))
        })
        .collect::<Vec<_>>();

    for (range, value) in replace_list.into_iter().rev() {
        replaced.replace_range(range, &value);
    }
    replaced
}

#[cfg(test)]
pub mod test {
    use std::collections::HashSet;
    use crate::compiler::{ModuleMeta, Compiler, replace_u_literal, str_xxhash};
    use libra::libra_types::language_storage::ModuleId;
    use libra::libra_types::account_address::AccountAddress;
    use ds::MockDataSource;
    use anyhow::Error;
    use libra::libra_vm::file_format::CompiledScript;
    use libra::move_core_types::identifier::Identifier;
    use crate::stdlib::zero_sdt;

    pub fn compile(
        source: &str,
        dep_list: Vec<(&str, &AccountAddress)>,
        address: &AccountAddress,
    ) -> Result<Vec<u8>, Error> {
        let ds = MockDataSource::with_write_set(zero_sdt());
        let compiler = Compiler::new(ds.clone());
        for (code, address) in dep_list {
            ds.publish_module(compiler.compile(code, address)?)?;
        }

        compiler.compile(source, address)
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
    fn test_create_compiler() {
        let view = MockDataSource::new();
        let _compiler = Compiler::new(view);
    }

    #[test]
    fn test_move_meta() {
        let view = MockDataSource::new();
        let compiler = Compiler::new(view);
        let meta = compiler
            .code_meta(&include_str!(
                "../../tests/resources/transaction_fee_distribution.move"
            ))
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
    fn test_u_literal() {
        assert_eq!(
            replace_u_literal(
                "Oracle.get_price(#\"USD\") + Oracle.get_price(#\"BTC\") = #\"USDBTC\"",
            ),
            format!(
                "Oracle.get_price({}) + Oracle.get_price({}) = {}",
                str_xxhash("usd"),
                str_xxhash("btc"),
                str_xxhash("usdbtc")
            )
        )
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
}
