mod imports;
mod name_pull;

use libra::libra_state_view::StateView;
use libra::libra_types::account_address::AccountAddress;
use libra::bytecode_verifier::{VerifiedModule, VerifiedScript};
use libra::move_lang::parser::ast::*;
use std::collections::HashMap;
use std::convert::TryFrom;

use anyhow::Error;
use libra::move_lang::{
    parser, parser::syntax::parse_file_string, shared::Address, strip_comments_and_verify,
    compile_program,
};
use libra::move_lang::errors::{Errors, report_errors_to_buffer};
use std::iter::FromIterator;
use libra::libra_vm::CompiledModule;
use libra::libra_types::language_storage::ModuleId;
use crate::compiler::module_loader::ModuleLoader;
use crate::compiler::{ModuleMeta, Builder, replace_u_literal};
use crate::banch32::replace_bech32_addresses;
use libra::libra_vm::file_format::CompiledScript;
use libra::move_lang::compiled_unit::CompiledUnit;
use crate::compiler::mv::imports::ImportsExtractor;
use crate::compiler::mv::name_pull::NamePull;

#[derive(Clone)]
pub struct Move<S>
where
    S: StateView + Clone,
{
    loader: ModuleLoader<S>,
}

impl<S> Move<S>
where
    S: StateView + Clone,
{
    pub fn new(module_loader: ModuleLoader<S>) -> Move<S> {
        Move {
            loader: module_loader,
        }
    }

    fn extract_meta(file_definition: &FileDefinition) -> Result<ModuleMeta, Error> {
        let module_name = match file_definition {
            FileDefinition::Modules(deps) => deps
                .iter()
                .find_map(|dep| match dep {
                    ModuleOrAddress::Address(_, _) => None,
                    ModuleOrAddress::Module(m) => Some(m.name.0.value.to_owned()),
                })
                .unwrap_or_else(|| "unknown".to_owned()),
            FileDefinition::Main(_main) => "main".to_owned(),
        };

        let mut extractor = ImportsExtractor::default();
        extractor.extract(&file_definition)?;

        Ok(ModuleMeta {
            module_name,
            dep_list: Vec::from_iter(extractor.imports().into_iter()),
        })
    }

    fn compile(&self, source: &str, address: &AccountAddress) -> Result<CompiledUnit, Error> {
        let address = Address::try_from(address.as_ref()).map_err(Error::msg)?;
        let mut source_map = HashMap::new();
        let pprog_res = self.parse_program(&source, &mut source_map)?;
        let mut prog = compile_program(pprog_res, Some(address)).map_err(|errs| {
            source_map.insert("source", source.to_string());
            error_render(errs, source_map)
        })?;
        Ok(prog.remove(0))
    }

    fn parse_program(
        &self,
        source: &str,
        source_map: &mut HashMap<&'static str, String>,
    ) -> Result<Result<parser::ast::Program, Errors>, Error> {
        let mut errors: Errors = Vec::new();

        let (def_opt, mut es) = parse_module(source, "source")?;
        errors.append(&mut es);

        let res = if errors.is_empty() {
            let definition = def_opt.ok_or_else(|| Error::msg("Unit not defined"))?;
            let mut modules = HashMap::new();
            let mut name_pull = NamePull::new();
            self.load_dependency(&definition, source_map, &mut modules, &mut name_pull)?;
            let lib_definitions = modules.into_iter().map(|(_, v)| v).collect();

            Ok(parser::ast::Program {
                source_definitions: vec![definition],
                lib_definitions,
            })
        } else {
            Err(errors)
        };
        Ok(res)
    }

    fn load_dependency(
        &self,
        definition: &FileDefinition,
        source_map: &mut HashMap<&'static str, String>,
        definition_map: &mut HashMap<ModuleId, FileDefinition>,
        name_pull: &mut NamePull,
    ) -> Result<(), Error> {
        let meta = Self::extract_meta(definition)?;
        let dep_list = self.loader.load_modules_signature(&meta.dep_list)?;
        for signature in dep_list {
            let source = signature.to_string();
            let name = name_pull
                .next()
                .ok_or_else(|| Error::msg("name pool depleted"))?;

            let def = self.make_file_definition(&source, name)?;
            self.load_dependency(&def, source_map, definition_map, name_pull)?;

            source_map.insert(name, source);
            definition_map.insert(signature.self_id().clone(), def);
        }
        Ok(())
    }

    fn make_file_definition(
        &self,
        source: &str,
        name: &'static str,
    ) -> Result<FileDefinition, Error> {
        let (def, err) = parse_module(source, name)?;
        if !err.is_empty() {
            Err(Error::msg(format!("Failed to parse module{:?};", err)))
        } else {
            def.ok_or_else(|| Error::msg("Expected module".to_string()))
        }
    }

    fn compile_unit(&self, code: &str, address: &AccountAddress) -> Result<Vec<u8>, Error> {
        let code = pre_processing(code);
        let unit = self.compile(&code, address)?;
        Ok(unit.serialize())
    }
}

impl<S> Builder for Move<S>
where
    S: StateView + Clone,
{
    fn build_module(&self, code: &str, address: &AccountAddress) -> Result<Vec<u8>, Error> {
        let unit = self.compile_unit(code, address)?;
        VerifiedModule::new(CompiledModule::deserialize(&unit)?)
            .map_err(|err| Error::msg(format!("Verification error: {:?}", err)))?;
        Ok(unit)
    }

    fn build_script(&self, code: &str, address: &AccountAddress) -> Result<Vec<u8>, Error> {
        let unit = self.compile_unit(code, address)?;
        VerifiedScript::new(CompiledScript::deserialize(&unit)?)
            .map_err(|err| Error::msg(format!("Verification error: {:?}", err)))?;
        Ok(unit)
    }

    fn module_meta(&self, code: &str) -> Result<ModuleMeta, Error> {
        let code = pre_processing(code);
        let file_definition = parse_module(&code, "mod")?
            .0
            .ok_or_else(|| Error::msg("Unexpected error"))?;

        Self::extract_meta(&file_definition)
    }
}

fn pre_processing(code: &str) -> String {
    let code = replace_bech32_addresses(code);
    replace_u_literal(&code)
}

pub fn parse_module(
    src: &str,
    name: &'static str,
) -> Result<(Option<parser::ast::FileDefinition>, Errors), Error> {
    let mut errors: Errors = Vec::new();

    let no_comments_buffer = match strip_comments_and_verify(name, src) {
        Err(err) => {
            errors.push(err);
            return Ok((None, errors));
        }
        Ok(no_comments_buffer) => no_comments_buffer,
    };
    let def_opt = match parse_file_string(name, &no_comments_buffer) {
        Ok(def) => Some(def),
        Err(err) => {
            errors.push(err);
            None
        }
    };
    Ok((def_opt, errors))
}

pub fn error_render(errors: Errors, source_map: HashMap<&'static str, String>) -> Error {
    match String::from_utf8(report_errors_to_buffer(source_map, errors)) {
        Ok(error) => Error::msg(error),
        Err(err) => Error::new(err),
    }
}

#[cfg(test)]
mod test {
    use libra::{libra_vm, libra_types};
    use libra_types::account_address::AccountAddress;
    use libra_vm::access::ModuleAccess;
    use libra_vm::CompiledModule;
    use crate::compiler::test::{compile, make_address};
    use libra_vm::file_format::CompiledScript;

    #[test]
    pub fn test_build_module_success() {
        let program = "module M {}";
        compile(program, vec![], &AccountAddress::random()).unwrap();
    }

    #[test]
    pub fn test_build_module_failed() {
        let program = "module M {";
        let error = compile(program, vec![], &AccountAddress::random())
            .err()
            .unwrap();
        assert!(error.to_string().contains("Unexpected end-of-file"));
    }

    #[test]
    pub fn test_build_script() {
        let program = "fun main() {}";
        compile(program, vec![], &AccountAddress::random()).unwrap();
    }

    #[test]
    pub fn test_build_script_with_dependence() {
        let dep = "\
        module M {
            public fun foo(): u64 {
                1
            }
        }
        ";
        let program = "\
        fun main() {\
            0x1::M::foo();
        }";

        compile(
            program,
            vec![(dep, &make_address("0x1"))],
            &AccountAddress::random(),
        )
        .unwrap();
    }

    #[test]
    fn test_parse_mvir_script_with_bech32_addresses() {
        let dep = r"
            module Account {}
        ";

        let program = r"
            import df1pfk58n7j62uenmam7f9ncu6qnffc2q5dpwuute.Account;
            main() {
                return;
            }
        ";

        let script = compile(
            program,
            vec![(
                dep,
                &make_address("0x646600000a6d43cfd2d2b999efbbf24b3c73409a5385028d"),
            )],
            &AccountAddress::default(),
        )
        .unwrap();

        let script = CompiledScript::deserialize(&script)
            .unwrap()
            .into_module()
            .1;
        let module = script
            .module_handles()
            .iter()
            .find(|h| script.identifier_at(h.name).to_string() == "Account")
            .unwrap();
        let address = script.address_identifier_at(module.address);
        assert_eq!(
            address.to_string(),
            "646600000a6d43cfd2d2b999efbbf24b3c73409a5385028d"
        );
    }

    #[test]
    fn test_parse_mvir_module_with_bech32_addresses() {
        let dep = r"
            module Account {}
        ";

        let program = r"
            module M {
                import df1pfk58n7j62uenmam7f9ncu6qnffc2q5dpwuute.Account;
            }
        ";

        let main_module = compile(
            program,
            vec![(
                dep,
                &make_address("0x646600000a6d43cfd2d2b999efbbf24b3c73409a5385028d"),
            )],
            &AccountAddress::default(),
        )
        .unwrap();
        let main_module = CompiledModule::deserialize(&main_module).unwrap();

        let module = main_module
            .module_handles()
            .iter()
            .find(|h| main_module.identifier_at(h.name).to_string() == "Account")
            .unwrap();
        let address = main_module.address_identifier_at(module.address);
        assert_eq!(
            address.to_string(),
            "646600000a6d43cfd2d2b999efbbf24b3c73409a5385028d"
        );
    }
}
