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
        if self.may_be_mvir(code) {
            self.build_with_compiler(code, address, &self.mvir)
                .or_else(|err| {
                    self.build_with_compiler(code, address, &self.mv)
                        .map_err(|_| err)
                })
        } else {
            self.build_with_compiler(code, address, &self.mv)
                .or_else(|err| {
                    self.build_with_compiler(code, address, &self.mvir)
                        .map_err(|_| err)
                })
        }
    }

    pub fn code_meta(&self, code: &str) -> Result<ModuleMeta, Error> {
        if self.may_be_mvir(code) {
            self.mvir
                .module_meta(code)
                .or_else(|err| self.mv.module_meta(code).map_err(|_| err))
        } else {
            self.mv
                .module_meta(code)
                .or_else(|err| self.mvir.module_meta(code).map_err(|_| err))
        }
    }

    fn may_be_mvir(&self, code: &str) -> bool {
        (code.contains("import") || code.contains("move") || code.contains("copy"))
            && !(code.contains("fun") && code.contains("use"))
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

#[cfg(test)]
pub mod test {
    use std::collections::HashSet;
    use crate::compiler::{Lang, ModuleMeta, Compiler};
    use libra::libra_types::language_storage::ModuleId;
    use libra::libra_types::account_address::AccountAddress;
    use libra::libra_types::identifier::Identifier;
    use ds::MockDataSource;
    use crate::stdlib::build_std;
    use anyhow::Error;
    use libra::vm::file_format::CompiledScript;

    pub fn compile(
        source: &str,
        dep: Option<(&str, &AccountAddress)>,
        address: &AccountAddress,
    ) -> Result<Vec<u8>, Error> {
        let ds = MockDataSource::with_write_set(build_std());
        let compiler = Compiler::new(ds.clone());
        if let Some((code, address)) = dep {
            ds.publish_module(compiler.compile(code, address)?)?;
        }

        compiler.compile(source, address)
    }

    pub fn compile_script(
        source: &str,
        dep: Option<(&str, &AccountAddress)>,
        address: &AccountAddress,
    ) -> CompiledScript {
        CompiledScript::deserialize(&compile(source, dep, address).unwrap()).unwrap()
    }

    pub fn make_address(address: &str) -> Result<AccountAddress, Error> {
        AccountAddress::from_hex_literal(address)
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
                    Identifier::new("LibraAccount").unwrap(),
                ),
                ModuleId::new(
                    AccountAddress::default(),
                    Identifier::new("LibraCoin").unwrap(),
                ),
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
    fn test_mvir_meta() {
        let view = MockDataSource::new();
        let compiler = Lang::MvIr.compiler(view);
        let meta = compiler
            .module_meta(&include_str!("../../stdlib/account.mvir"))
            .unwrap();
        assert_eq!(
            meta,
            ModuleMeta {
                module_name: "Account".to_string(),
                dep_list: vec![ModuleId::new(
                    AccountAddress::default(),
                    Identifier::new("Coins").unwrap(),
                ),],
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
                import 0x0.LibraAccount;
                import 0x0.LibraCoin;
                main(payee: address, amount: u64) {
                  LibraAccount.mint_to_address(move(payee), move(amount));
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
                        Identifier::new("LibraAccount").unwrap(),
                    ),
                    ModuleId::new(
                        AccountAddress::default(),
                        Identifier::new("LibraCoin").unwrap(),
                    ),
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

            use 0x0::WBCoins;

            fun main(payee: address, amount: u64) {
                0x0::LibraAccount::mint_to_address(payee, amount)
            }
            ",
            )
            .unwrap();
        assert_eq!(&meta.module_name, "main");
        assert_eq!(
            meta.dep_list.into_iter().collect::<HashSet<_>>(),
            vec![
                ModuleId::new(
                    AccountAddress::default(),
                    Identifier::new("WBCoins").unwrap(),
                ),
                ModuleId::new(
                    AccountAddress::default(),
                    Identifier::new("LibraAccount").unwrap(),
                ),
            ]
            .into_iter()
            .collect::<HashSet<_>>()
        );
    }

    #[test]
    fn test_build_move() {
        let compiler = Compiler::new(MockDataSource::with_write_set(build_std()));
        compiler
            .compile(
                "\
            use 0x0::BytearrayUtil;

            fun main(a1: bytearray, a2: bytearray) {
                BytearrayUtil::bytearray_concat(a1, a2);
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
