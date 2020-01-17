use std::collections::{HashMap, HashSet};
use std::fs::read_to_string;

use maplit::hashmap;
use bytecode_verifier::verifier::{VerifiedModule, VerifiedProgram};
use compiler::Compiler;
use libra_types::account_address::AccountAddress;
use structopt::StructOpt;
use vm::access::{ModuleAccess, ScriptAccess};
use vm::CompiledModule;
use vm::file_format::{CompiledProgram, CompiledScript};
use vm::internals::ModuleIndex;

#[derive(Debug, Eq, PartialEq, Hash)]
struct ImportedModule {
    address: AccountAddress,
    name: String,
}

fn extract_imported_modules_from_module(module: &CompiledModule) -> HashSet<ImportedModule> {
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

fn extract_imported_modules_from_script(script: &CompiledScript) -> HashSet<ImportedModule> {
    let address_pool = script.address_pool();
    let identifiers = script.identifiers();
    script
        .module_handles()
        .iter()
        .map(|handle| ImportedModule {
            address: address_pool[handle.address.into_index()],
            name: identifiers[handle.name.into_index()].to_string(),
        })
        .filter(|imp| imp.name != "<SELF>")
        .collect()
}

fn collect_imported_modules(program: &VerifiedProgram) -> HashSet<ImportedModule> {
    let mut used_module_handles: HashSet<ImportedModule> = HashSet::new();
    for module in program.modules() {
        used_module_handles.extend(extract_imported_modules_from_module(module.as_inner()));
    }
    used_module_handles.extend(extract_imported_modules_from_script(
        program.script().as_inner(),
    ));
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

#[derive(Debug)]
pub struct Whitelist {
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

pub fn is_only_whitelisted_imports(
    source: &str,
    address: AccountAddress,
    whitelist: Whitelist,
) -> bool {
    let imports = extract_imports(source, address);
    imports
        .iter()
        .all(|imp| imp.name == "<SELF>" || imp.address == address || whitelist.contains(imp))
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
    let whitelist = Whitelist::new(hashmap! {
        AccountAddress::default() => vec!["*".to_string()]
    });
    dbg!(is_only_whitelisted_imports(source, address, whitelist));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_whitelisted(
        source: &str,
        whitelist_mapping: HashMap<AccountAddress, Vec<String>>,
    ) -> bool {
        let address = AccountAddress::new([1; 32]);
        let whitelist = Whitelist::new(whitelist_mapping);
        is_only_whitelisted_imports(source, address, whitelist)
    }

    #[test]
    fn test_all_modules_are_whitelisted() {
        let source = r"
            import 0x0.LibraCoin;
            import 0x0.LibraAccount;

            main() {
                return;
            }
        ";
        let whitelist = hashmap! {
            AccountAddress::default() => vec![CATCH_ALL.to_string()]
        };
        assert!(is_whitelisted(source, whitelist));
    }

    #[test]
    fn test_specific_whitelisted_modules() {
        let source = r"
            import 0x0.LibraCoin;
            import 0x0.LibraAccount;

            main() {
                return;
            }
        ";
        let whitelist = hashmap! {
            AccountAddress::default() => vec!["LibraCoin".to_string(), "LibraAccount".to_string()]
        };
        assert!(is_whitelisted(source, whitelist));
    }

    #[test]
    fn test_locally_defined_module_always_whitelisted() {
        let source = r"
            module MyModule {
                import 0x0.LibraCoin;
                import 0x0.LibraAccount;
            }
        ";
        let whitelist = hashmap! {
            AccountAddress::default() => vec!["LibraCoin".to_string(), "LibraAccount".to_string()]
        };
        assert!(is_whitelisted(source, whitelist));
    }

    #[test]
    fn test_some_module_is_not_whitelisted() {
        let source = r"
            import 0x0.LibraCoin;
            import 0x0.LibraAccount;

            main() {
                return;
            }
        ";
        let whitelist = hashmap! {
            AccountAddress::default() => vec!["LibraCoin".to_string()]
        };
        assert!(!is_whitelisted(source, whitelist));
    }
}
