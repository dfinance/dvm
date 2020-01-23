use std::sync::Mutex;

use bytecode_verifier::VerifiedModule;
use compiler::Compiler as MvIrCompiler;
use libra_types::account_address::AccountAddress;

use crate::move_lang::{build_with_deps, Code, find_and_replace_bech32_addresses};

pub enum Lang {
    Move,
    MvIr,
}

impl Lang {
    pub fn compiler(&self) -> Box<dyn Compiler> {
        match self {
            Lang::Move => Box::new(Move::new()),
            Lang::MvIr => Box::new(MvIr::new()),
        }
    }
}

pub trait Compiler {
    fn build_module(&self, code: &str, address: &AccountAddress) -> Vec<u8>;
    fn build_script(&self, code: &str, address: &AccountAddress) -> Vec<u8>;
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
    fn build_module(&self, code: &str, address: &AccountAddress) -> Vec<u8> {
        let mut cache = self.cache.lock().unwrap();

        let deps = cache.iter().map(|dep| Code::module("dep", dep)).collect();
        let module = build_with_deps(Code::module("source", code), deps, address).unwrap();

        cache.push(code.to_owned());
        module.serialize()
    }

    fn build_script(&self, code: &str, address: &AccountAddress) -> Vec<u8> {
        let cache = self.cache.lock().unwrap();

        let deps = cache.iter().map(|dep| Code::module("dep", dep)).collect();

        let module = build_with_deps(Code::script(code), deps, address).unwrap();
        module.serialize()
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
    fn build_module(&self, code: &str, address: &AccountAddress) -> Vec<u8> {
        let code = find_and_replace_bech32_addresses(code);

        let mut cache = self.cache.lock().unwrap();
        let mut compiler = MvIrCompiler::default();

        compiler.extra_deps = cache.clone();
        compiler.address = *address;
        let module = compiler.into_compiled_module(&code).unwrap();
        let mut buff = Vec::new();
        module.serialize(&mut buff).unwrap();

        cache.push(VerifiedModule::new(module).unwrap());
        buff
    }

    fn build_script(&self, code: &str, address: &AccountAddress) -> Vec<u8> {
        let code = find_and_replace_bech32_addresses(code);

        let cache = self.cache.lock().unwrap();
        let mut compiler = MvIrCompiler::default();

        compiler.extra_deps = cache.clone();
        compiler.address = *address;
        let module = compiler.into_compiled_program(&code).unwrap();
        let mut buff = Vec::new();

        module.script.serialize(&mut buff).unwrap();
        buff
    }
}
