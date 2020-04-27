mod module_loader;
mod mv;
mod mvir;

pub use mv::Move;
pub use mvir::Mvir;

use libra::libra_state_view::StateView;
use libra::libra_types::account_address::AccountAddress;
use anyhow::Error;
use libra::libra_types::language_storage::ModuleId;
use crate::compiler::module_loader::ModuleLoader;
use crate::pattern;
use twox_hash::XxHash64;
use std::hash::Hasher;

pub enum Lang {
    Move,
    MvIr,
}

impl Lang {
    pub fn compiler<S>(&self, view: S) -> Box<dyn Builder>
    where
        S: StateView + Clone + 'static,
    {
        let loader = ModuleLoader::new(view);
        match self {
            Lang::Move => Box::new(mv(loader)),
            Lang::MvIr => Box::new(mvir(loader)),
        }
    }
}

#[derive(Clone)]
pub struct Compiler<S>
where
    S: StateView + Clone,
{
    mvir: Mvir<S>,
    mv: Move<S>,
}

impl<S> Compiler<S>
where
    S: StateView + Clone,
{
    pub fn new(view: S) -> Compiler<S> {
        let loader = ModuleLoader::new(view);
        Compiler {
            mvir: mvir(loader.clone()),
            mv: mv(loader),
        }
    }

    pub fn compile(&self, code: &str, address: &AccountAddress) -> Result<Vec<u8>, Error> {
        if self.is_move(code) {
            self.build_with_compiler(code, address, &self.mv)
        } else if self.is_mvir(code) {
            self.build_with_compiler(code, address, &self.mvir)
        } else {
            self.build_with_compiler(code, address, &self.mv)
                .or_else(|err| {
                    self.build_with_compiler(code, address, &self.mvir)
                        .map_err(|_| err)
                })
        }
    }

    pub fn code_meta(&self, code: &str) -> Result<ModuleMeta, Error> {
        if self.is_move(code) {
            self.mv.module_meta(code)
        } else if self.is_mvir(code) {
            self.mvir.module_meta(code)
        } else {
            self.mv
                .module_meta(code)
                .or_else(|err| self.mvir.module_meta(code).map_err(|_| err))
        }
    }

    fn is_move(&self, code: &str) -> bool {
        code.contains("fun") || code.contains("use") || code.contains("resource struct")
    }

    fn is_mvir(&self, code: &str) -> bool {
        code.contains("import") || code.contains("move(") || code.contains("copy(")
    }

    fn may_be_script(&self, code: &str) -> bool {
        code.contains("main") && !code.contains("module")
    }

    fn build_with_compiler<B>(
        &self,
        code: &str,
        address: &AccountAddress,
        compiler: &B,
    ) -> Result<Vec<u8>, Error>
    where
        B: Builder,
    {
        if self.may_be_script(code) {
            compiler.build_script(code, address)
        } else {
            compiler.build_module(code, address)
        }
    }
}

pub fn mv<S>(module_loader: ModuleLoader<S>) -> Move<S>
where
    S: StateView + Clone,
{
    Move::new(module_loader)
}

pub fn mvir<S>(module_loader: ModuleLoader<S>) -> Mvir<S>
where
    S: StateView + Clone,
{
    Mvir::new(module_loader)
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
    use crate::compiler::{Lang, ModuleMeta, Compiler, replace_u_literal, str_xxhash};
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
        let _compiler = Lang::Move.compiler(view.clone());
        let _compiler = Lang::MvIr.compiler(view);
    }

    #[test]
    fn test_move_meta() {
        let view = MockDataSource::new();
        let compiler = Lang::Move.compiler(view);
        let meta = compiler
            .module_meta(&include_str!(
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
    fn test_mvir_meta() {
        let view = MockDataSource::new();
        let compiler = Lang::MvIr.compiler(view);
        let meta = compiler
            .module_meta(&include_str!("../../tests/resources/account.mvir"))
            .unwrap();
        assert_eq!(
            meta,
            ModuleMeta {
                module_name: "Account".to_string(),
                dep_list: vec![
                    ModuleId::new(AccountAddress::default(), Identifier::new("Coins").unwrap()),
                    ModuleId::new(
                        AccountAddress::default(),
                        Identifier::new("AddressUtil").unwrap(),
                    ),
                    ModuleId::new(
                        AccountAddress::default(),
                        Identifier::new("U64Util").unwrap(),
                    ),
                    ModuleId::new(
                        AccountAddress::default(),
                        Identifier::new("BytearrayUtil").unwrap(),
                    ),
                ],
            }
        )
    }

    #[test]
    fn test_mvir_script_meta() {
        let view = MockDataSource::new();
        let compiler = Lang::MvIr.compiler(view);
        let meta = compiler
            .module_meta(
                "
                import 0x0.Account;
                import 0x0.Coin;
                main(payee: address, amount: u64) {
                  Account.mint_to_address(move(payee), move(amount));
                  return;
                }
            ",
            )
            .unwrap();
        assert_eq!(
            meta,
            ModuleMeta {
                module_name: "main".to_string(),
                dep_list: vec![
                    ModuleId::new(
                        AccountAddress::default(),
                        Identifier::new("Account").unwrap(),
                    ),
                    ModuleId::new(AccountAddress::default(), Identifier::new("Coin").unwrap()),
                ],
            }
        )
    }

    #[test]
    fn test_move_script_meta() {
        let view = MockDataSource::new();
        let compiler = Lang::Move.compiler(view);
        let meta = compiler
            .module_meta(
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

    #[test]
    fn test_combine_compilation() {
        let view = MockDataSource::new();
        let compiler = Compiler::new(view.clone());

        let addr = AccountAddress::default();

        let module = compiler
            .compile(
                include_str!("../../tests/resources/move_to_mvir/r.move"),
                &addr,
            )
            .unwrap();
        view.publish_module(module).unwrap();

        let module = compiler
            .compile(
                include_str!("../../tests/resources/move_to_mvir/s.mvir"),
                &addr,
            )
            .unwrap();
        view.publish_module(module).unwrap();

        let module = compiler
            .compile(
                include_str!("../../tests/resources/move_to_mvir/c.mvir"),
                &addr,
            )
            .unwrap();
        view.publish_module(module).unwrap();

        let module = compiler
            .compile(
                include_str!("../../tests/resources/move_to_mvir/c_wrapper.move"),
                &addr,
            )
            .unwrap();
        view.publish_module(module).unwrap();

        let module = compiler
            .compile(
                include_str!("../../tests/resources/move_to_mvir/r_wrapper.move"),
                &addr,
            )
            .unwrap();
        view.publish_module(module).unwrap();

        let module = compiler
            .compile(
                include_str!("../../tests/resources/move_to_mvir/s_wrapper.move"),
                &addr,
            )
            .unwrap();
        view.publish_module(module).unwrap();
    }
}
