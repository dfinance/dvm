use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Error, Formatter};

use anyhow::Result;
use maplit::hashmap;

use libra::{libra_types, vm};
use libra_types::account_address::AccountAddress;
use vm::access::ScriptAccess;
use vm::file_format::{CompiledScript, ModuleHandle};

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
    let address = script.address_at(handle.address).to_owned();
    let name = script.identifier_at(handle.name).to_string();
    ImportedModule { address, name }
}

pub struct WhitelistVerifier {
    allowed_modules: HashMap<AccountAddress, HashSet<String>>,
}

impl WhitelistVerifier {
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
mod tests {
    use super::*;
    use crate::compiler::test::{compile_script, make_address};

    fn verify_source_code(
        source: &str,
        dep: Option<(&str, &AccountAddress)>,
        verifier: WhitelistVerifier,
        sender_address: AccountAddress,
    ) -> Result<()> {
        let compiled = compile_script(source, dep, &sender_address);
        verifier.verify_only_whitelisted_modules(&compiled)
    }

    #[test]
    fn test_all_modules_are_whitelisted() {
        let sender_address =
            make_address("0x123456789abcdef123456789abcdef123456789abcdef123456789abcdefeeee")
                .unwrap();
        let source = r"
            import 0x0.Coins;
            import 0x0.Account;

            main() {
                return;
            }
        ";
        let whitelist = hashmap! {
            AccountAddress::default() => vec!["Account".to_string(), "Coins".to_string()]
        };
        let verifier = WhitelistVerifier::new(sender_address, vec![], whitelist);

        verify_source_code(source, None, verifier, sender_address).unwrap()
    }

    #[test]
    fn test_modules_from_sender_address_not_flagged() {
        let sender_address =
            make_address("0x123456789abcdef123456789abcdef123456789abcdef123456789abcdefeeee")
                .unwrap();

        let dep = r"
            module Account {
            }
        ";
        let source = r"
            import 0x123456789abcdef123456789abcdef123456789abcdef123456789abcdefeeee.Account;
            main() {
                return;
            }
        ";
        let verifier =
            WhitelistVerifier::new(sender_address, vec!["Account".to_string()], hashmap! {});

        verify_source_code(
            source,
            Some((dep, &sender_address)),
            verifier,
            sender_address,
        )
        .unwrap()
    }

    #[test]
    fn test_module_on_sender_does_not_exist() {
        let sender_address =
            make_address("0x123456789abcdef123456789abcdef123456789abcdef123456789abcdefeeee")
                .unwrap();

        let dep = r"
            module Unknown {
            }
        ";

        let source = r"
            import 0x123456789abcdef123456789abcdef123456789abcdef123456789abcdefeeee.Unknown;
            main() {
                return;
            }
        ";
        let verifier =
            WhitelistVerifier::new(sender_address, vec!["Account".to_string()], hashmap! {});

        let err = verify_source_code(
            source,
            Some((dep, &sender_address)),
            verifier,
            sender_address,
        )
        .unwrap_err();
        assert_eq!(
            err.to_string(),
            "Module 123456789abcdef123456789abcdef123456789abcdef123456789abcdefeeee.Unknown is not whitelisted"
        );
    }

    #[test]
    fn test_some_module_is_not_whitelisted() {
        let sender_address =
            make_address("0x123456789abcdef123456789abcdef123456789abcdef123456789abcdefeeee")
                .unwrap();
        let source = r"
                import 0x0.Coins;
                import 0x0.Account;

                main() {
                    return;
                }
            ";
        let whitelist = hashmap! {
            AccountAddress::default() => vec!["Coins".to_string()]
        };
        let verifier =
            WhitelistVerifier::new(sender_address, vec!["Account".to_string()], whitelist);
        let verified_err = verify_source_code(source, None, verifier, sender_address).unwrap_err();
        assert_eq!(
            verified_err.to_string(),
            "Module 0000000000000000000000000000000000000000000000000000000000000000.Account is not whitelisted"
        );
    }
}
