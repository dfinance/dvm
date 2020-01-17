use std::collections::{HashMap, HashSet};
use std::fs::read_to_string;

use bytecode_verifier::verifier::{VerifiedModule, VerifiedProgram};
use compiler::Compiler;
use libra_types::account_address::AccountAddress;
use structopt::StructOpt;
use vm::access::ModuleAccess;
use vm::CompiledModule;
use vm::file_format::CompiledProgram;
use vm::internals::ModuleIndex;

#[derive(Debug, Eq, PartialEq, Hash)]
struct ImportedModule {
    address: AccountAddress,
    name: String,
}

fn extract_imported_modules(module: &CompiledModule) -> HashSet<ImportedModule> {
    let address_pool = module.address_pool();
    let identifiers = module.identifiers();
    module
        .module_handles()
        .iter()
        .map(|handle| ImportedModule {
            address: address_pool[handle.address.into_index()],
            name: identifiers[handle.name.into_index()].to_string(),
        })
        .collect()
}

fn collect_imported_modules(program: &VerifiedProgram) -> HashSet<ImportedModule> {
    let mut used_module_handles: HashSet<ImportedModule> = HashSet::new();
    for module in program.modules() {
        used_module_handles.extend(extract_imported_modules(module.as_inner()));
    }
    used_module_handles
}

fn compile_source(source: &str, address: AccountAddress) -> (CompiledProgram, Vec<VerifiedModule>) {
    let compiler = Compiler {
        address,
        skip_stdlib_deps: false,
        ..Compiler::default()
    };
    compiler
        .into_compiled_program_and_deps(&source)
        .expect("Failed to compile program")
}

fn extract_imports(source: &str, address: AccountAddress) -> HashSet<ImportedModule> {
    let (compiled, deps) = compile_source(source, address);
    let verified_program =
        VerifiedProgram::new(compiled, &deps[..]).expect("Failed to verify program");

    collect_imported_modules(&verified_program)
}

struct Whitelist {
    mapping: HashMap<AccountAddress, Vec<String>>,
}

const CATCH_ALL: &str = "*";

impl Whitelist {
    fn new(mapping: HashMap<AccountAddress, Vec<String>>) -> Self {
        Whitelist { mapping }
    }

    fn contains(&self, imp: &ImportedModule) -> bool {
        match self.mapping.get(&imp.address) {
            Some(names) => names.contains(&CATCH_ALL.to_string()) || names.contains(&imp.name),
            None => false,
        }
    }
}

#[derive(StructOpt)]
struct Opts {
    fname: String,
}

fn main() {
    let Opts { fname } = Opts::from_args();
    let source = Box::leak(
        read_to_string(fname)
            .expect("Unable to read file")
            .into_boxed_str(),
    );
    let address = AccountAddress::default();
    let mut mapping = HashMap::new();
    mapping.insert(AccountAddress::default(), vec!["*".to_string()]);

    let whitelist = Whitelist::new(mapping);

    let imports = extract_imports(source, address);
    for import in imports.iter() {
        println!();
        println!("Module: '{}'", import.name);
        println!("Address: 0x{:#x}", import.address);
    }

    let has_non_whitelisted_module = imports.iter().any(|imp| !whitelist.contains(imp));
    dbg!(has_non_whitelisted_module);
}
