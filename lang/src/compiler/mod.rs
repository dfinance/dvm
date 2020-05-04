pub mod imports;
pub mod meta;
pub mod module_loader;
pub mod mv;
pub mod name_pull;
pub mod preprocessor;

use libra::libra_state_view::StateView;
use libra::libra_types::account_address::AccountAddress;
use anyhow::Error;
use crate::compiler::module_loader::ModuleLoader;
use std::collections::HashMap;
use crate::compiler::preprocessor::pre_processing;
use crate::compiler::mv::Move;
use crate::compiler::meta::{ModuleMeta, extract_meta};
use libra::move_lang::shared::Address;
use std::convert::TryFrom;
use crate::stdlib::zero_sdt;
use libra::libra_vm::file_format::CompiledScript;
use ds::MockDataSource;

#[derive(Clone)]
pub struct Compiler<S>
    where
        S: StateView + Clone,
{
    loader: ModuleLoader<S>,
}

impl<S> Compiler<S>
    where
        S: StateView + Clone,
{
    pub fn new(view: S) -> Compiler<S> {
        Compiler {
            loader: ModuleLoader::new(view),
        }
    }

    pub fn compile_source_map(
        &self,
        source_map: HashMap<&str, &str>,
        address: &AccountAddress,
    ) -> Result<HashMap<String, Vec<u8>>, Error> {
        let address = Address::try_from(address.as_ref()).map_err(Error::msg)?;
        let mut lang = Move::new(&self.loader);
        lang.compile_source_map(source_map, address)
    }

    pub fn compile(&self, code: &str, address: &AccountAddress) -> Result<Vec<u8>, Error> {
        let mut source_map = HashMap::new();
        source_map.insert("source", code);
        let bytecode_map = self.compile_source_map(source_map, address)?;
        bytecode_map
            .into_iter()
            .next()
            .map(|(_, bytecode)| bytecode)
            .ok_or_else(|| Error::msg("Expected source map is not empty."))
    }

    pub fn code_meta(&self, code: &str) -> Result<ModuleMeta, Error> {
        let code = pre_processing(code);

        let file_definition = Move::<S>::parse_module(&code, "mod")?
            .0
            .ok_or_else(|| Error::msg("Unexpected error"))?;
        extract_meta(&file_definition)
    }
}

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
