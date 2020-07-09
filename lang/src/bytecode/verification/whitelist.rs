use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Error, Formatter};

use anyhow::Result;
use maplit::hashmap;

use libra::prelude::*;

#[derive(Debug, Eq, PartialEq, Hash)]
struct ImportedModule {
    address: AccountAddress,
    name: String,
}

impl Display for ImportedModule {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        f.write_str(&format!("{}.{}", &self.address.to_string(), &self.name))
            .unwrap();
        Ok(())
    }
}

fn get_imported_module(script: &CompiledScript, handle: &ModuleHandle) -> ImportedModule {
    let address = *script.address_identifier_at(handle.address);
    let name = script.identifier_at(handle.name).to_string();
    ImportedModule { address, name }
}

/// Restricts set of modules allowed to use in script.
pub struct WhitelistVerifier {
    allowed_modules: HashMap<AccountAddress, HashSet<String>>,
}

impl WhitelistVerifier {
    /// Only modules allowed to use are modules from whitelist and owner's modules.
    pub fn new(
        sender_address: AccountAddress,
        sender_modules: Vec<String>,
        whitelisted_modules: HashMap<AccountAddress, Vec<String>>,
    ) -> Self {
        let mut allowed_modules: HashMap<AccountAddress, HashSet<String>> = hashmap! {};
        for (address, modules) in whitelisted_modules {
            allowed_modules.insert(address, modules.iter().map(|s| s.to_owned()).collect());
        }
        allowed_modules.insert(
            sender_address,
            sender_modules.iter().map(String::to_owned).collect(),
        );
        WhitelistVerifier { allowed_modules }
    }

    /// Verify whether all `use` statements in script importing only modules from whitelist.
    pub fn verify_only_whitelisted_modules(&self, script: &CompiledScript) -> Result<()> {
        let deps: HashSet<ImportedModule> = script
            .module_handles()
            .iter()
            .map(|handle| get_imported_module(script, handle))
            .collect();
        for module_dep in deps {
            if module_dep.name == "<SELF>" {
                continue;
            }
            let allowed_modules = self.allowed_modules.get(&module_dep.address);
            match allowed_modules {
                None => bail!("Address {} is not whitelisted", module_dep.address),
                Some(allowed_modules) => ensure!(
                    allowed_modules.contains(&module_dep.name),
                    "Module {} is not whitelisted",
                    module_dep
                ),
            }
        }
        Ok(())
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use libra::libra_types::account_address::AccountAddress;
    use compiler::Compiler;
    use anyhow::Error;
    use ds::MockDataSource;
    use libra::move_core_types::language_storage::CORE_CODE_ADDRESS;

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

    fn verify_source_code(
        source: &str,
        dep: Vec<(&str, &AccountAddress)>,
        verifier: WhitelistVerifier,
        sender_address: AccountAddress,
    ) -> Result<()> {
        let compiled = compile_script(source, dep, &sender_address);
        verifier.verify_only_whitelisted_modules(&compiled)
    }

    #[test]
    fn test_all_modules_are_whitelisted() {
        let sender_address = make_address("0x646600000a6d43cfd2d2b999efbbf24b3c73409a");

        let empty = include_str!("../../../tests/resources/empty.move");
        let oracle = include_str!("../../../tests/resources/debug.move");

        let source = "
            script {
            use 0x1::Empty;
            use 0x1::Debug;

            fun main() {
                Empty::create();
                Debug::print_stack_trace();
            }
            }
        ";
        let whitelist = hashmap! {
            CORE_CODE_ADDRESS => vec!["Empty".to_string(), "Debug".to_string()]
        };
        let verifier = WhitelistVerifier::new(sender_address, vec![], whitelist);

        let core = CORE_CODE_ADDRESS;
        verify_source_code(
            source,
            vec![(empty, &core), (oracle, &core)],
            verifier,
            sender_address,
        )
        .unwrap()
    }

    #[test]
    fn test_modules_from_sender_address_not_flagged() {
        let sender_address = make_address("0x646600000a6d43cfd2d2b999efbbf24b3c73409a");

        let dep = r"
            module Account {
                public fun foo() {}
            }
        ";
        let source = r"
            script {
            use 0x646600000a6d43cfd2d2b999efbbf24b3c73409a::Account;
            fun main() {
                Account::foo();
            }
            }
        ";
        let verifier =
            WhitelistVerifier::new(sender_address, vec!["Account".to_string()], hashmap! {});

        verify_source_code(
            source,
            vec![(dep, &sender_address)],
            verifier,
            sender_address,
        )
        .unwrap()
    }

    #[test]
    fn test_module_on_sender_does_not_exist() {
        let sender_address = make_address("0x646600000a6d43cfd2d2b999efbbf24b3c73409a");

        let dep = r"
            module Unknown {
                public fun foo(){}
            }
        ";

        let source = r"
            script {
            use 0x646600000a6d43cfd2d2b999efbbf24b3c73409a::Unknown;
            fun main() {
                Unknown::foo();
            }
            }
        ";
        let verifier =
            WhitelistVerifier::new(sender_address, vec!["Account".to_string()], hashmap! {});

        let err = verify_source_code(
            source,
            vec![(dep, &sender_address)],
            verifier,
            sender_address,
        )
        .unwrap_err();
        assert_eq!(
            err.to_string(),
            "Module 646600000a6d43cfd2d2b999efbbf24b3c73409a.Unknown is not whitelisted"
        );
    }

    #[test]
    fn test_some_module_is_not_whitelisted() {
        let sender_address = make_address("0x646600000a6d43cfd2d2b999efbbf24b3c73409a");
        let empty = include_str!("../../../tests/resources/empty.move");
        let oracle = include_str!("../../../tests/resources/debug.move");

        let source = "
                 script {
                 use 0x1::Empty;
                 use 0x1::Debug;

                fun main() {
                    Empty::create();
                    Debug::print_stack_trace();
                }
                }
            ";
        let whitelist = hashmap! {
            CORE_CODE_ADDRESS => vec!["Debug".to_string()]
        };
        let core = CORE_CODE_ADDRESS;
        let verifier = WhitelistVerifier::new(sender_address, vec!["Debug".to_string()], whitelist);
        let verified_err = verify_source_code(
            source,
            vec![(empty, &core), (oracle, &core)],
            verifier,
            sender_address,
        )
        .unwrap_err();
        assert_eq!(
            verified_err.to_string(),
            "Module 0000000000000000000000000000000000000001.Empty is not whitelisted"
        );
    }
}
