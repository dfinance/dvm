pub mod mv;

use std::sync::Mutex;

use bytecode_verifier::VerifiedModule;
use compiler::Compiler as MvIrCompiler;
use libra_types::account_address::AccountAddress;
use anyhow::Error;
use std::str::FromStr;
use ir_to_bytecode::parser::parse_program;
use move_lang::parser::ast::{FileDefinition, ModuleOrAddress};
use crate::vm::compiler::mv::{build_with_deps, Code, parse_module};
use crate::vm::bech32_utils;

pub enum Lang {
    Move,
    MvIr,
}

impl FromStr for Lang {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "move" | "mv" => Ok(Lang::Move),
            "mvir" | "ir" => Ok(Lang::MvIr),
            _ => Err(Error::msg(format!("Unknown compiler type: {}", s))),
        }
    }
}

impl Lang {
    pub fn compiler(&self) -> Box<dyn Compiler> {
        match self {
            Lang::Move => Box::new(Move::new()),
            Lang::MvIr => Box::new(MvIr::new()),
        }
    }
}

#[derive(Debug)]
pub struct ModuleMeta {
    pub module_name: String,
    pub dep_list: Vec<String>,
}

pub trait Compiler {
    fn build_module(
        &self,
        code: &str,
        address: &AccountAddress,
        disable_std: bool,
    ) -> Result<Vec<u8>, Error>;
    fn build_script(
        &self,
        code: &str,
        address: &AccountAddress,
        disable_std: bool,
    ) -> Result<Vec<u8>, Error>;
    fn module_meta(&self, code: &str) -> Result<ModuleMeta, Error>;
}

struct Move {
    cache: Mutex<Vec<String>>,
}

impl Move {
    fn new() -> Move {
        Move {
            cache: Mutex::new(vec![]),
        }
    }
}

impl Compiler for Move {
    fn build_module(
        &self,
        code: &str,
        address: &AccountAddress,
        disable_std: bool,
    ) -> Result<Vec<u8>, Error> {
        let mut cache = self.cache.lock().unwrap();

        let deps = cache.iter().map(|dep| Code::module("dep", dep)).collect();
        let module = build_with_deps(Code::module("source", code), deps, address, disable_std)?;

        cache.push(code.to_owned());
        Ok(module.serialize())
    }

    fn build_script(
        &self,
        code: &str,
        address: &AccountAddress,
        disable_std: bool,
    ) -> Result<Vec<u8>, Error> {
        let cache = self.cache.lock().unwrap();

        let deps = cache.iter().map(|dep| Code::module("dep", dep)).collect();

        let module = build_with_deps(Code::script(code), deps, address, disable_std)?;
        Ok(module.serialize())
    }

    fn module_meta(&self, code: &str) -> Result<ModuleMeta, Error> {
        let module = parse_module(code, "mod")?
            .0
            .ok_or_else(|| Error::msg("Unexpected error"))?;

        match module {
            FileDefinition::Modules(deps) => {
                for dep in deps {
                    match &dep {
                        ModuleOrAddress::Module(module) => {
                            let dep_list = module
                                .uses
                                .iter()
                                .map(|(i, _)| i.0.value.name.0.value.clone())
                                .collect::<Vec<_>>();

                            return Ok(ModuleMeta {
                                module_name: module.name.0.to_string(),
                                dep_list,
                            });
                        }
                        ModuleOrAddress::Address(_, _) => {}
                    }
                }
                Err(Error::msg("Expected module"))
            }
            FileDefinition::Main(_main) => Err(Error::msg("Expected module")),
        }
    }
}

struct MvIr {
    cache: Mutex<Vec<VerifiedModule>>,
}

impl MvIr {
    fn new() -> MvIr {
        MvIr {
            cache: Mutex::new(vec![]),
        }
    }
}

impl Compiler for MvIr {
    fn build_module(
        &self,
        code: &str,
        address: &AccountAddress,
        disabled_std: bool,
    ) -> Result<Vec<u8>, Error> {
        let code = bech32_utils::find_and_replace_bech32_addresses(code);

        let mut cache = self.cache.lock().unwrap();
        let mut compiler = MvIrCompiler::default();
        compiler.skip_stdlib_deps = disabled_std;
        compiler.extra_deps = cache.clone();
        compiler.address = *address;
        let module = compiler.into_compiled_module(&code)?;
        let mut buff = Vec::new();
        module.serialize(&mut buff).unwrap();

        cache.push(VerifiedModule::new(module).map_err(|(_, s)| Error::msg(format!("{:?}", s)))?);
        Ok(buff)
    }

    fn build_script(
        &self,
        code: &str,
        address: &AccountAddress,
        disabled_std: bool,
    ) -> Result<Vec<u8>, Error> {
        let code = bech32_utils::find_and_replace_bech32_addresses(code);

        let cache = self.cache.lock().unwrap();
        let mut compiler = MvIrCompiler::default();
        compiler.skip_stdlib_deps = disabled_std;
        compiler.extra_deps = cache.clone();
        compiler.address = *address;
        let module = compiler.into_compiled_program(&code)?;
        let mut buff = Vec::new();

        module.script.serialize(&mut buff)?;
        Ok(buff)
    }

    fn module_meta(&self, code: &str) -> Result<ModuleMeta, Error> {
        let parsed_program = parse_program(code)?;
        let module = parsed_program
            .modules
            .get(0)
            .ok_or_else(|| Error::msg("Expected module."))?;

        let dep_list = module
            .imports
            .iter()
            .map(|i| i.ident.name().clone().into_inner().into_string())
            .collect();

        Ok(ModuleMeta {
            module_name: module.name.to_string(),
            dep_list,
        })
    }
}
