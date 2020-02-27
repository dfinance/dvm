use std::collections::HashMap;
use std::convert::TryFrom;

use anyhow::{Error, Result};
use libra_types::account_address::AccountAddress;
use move_lang::{
    compile_program, parser, parser::syntax::parse_file_string, shared::Address, stdlib,
    strip_comments_and_verify,
};
use move_lang::errors::{Errors, report_errors_to_buffer};
use move_lang::to_bytecode::translate::CompiledUnit;

#[derive(Debug)]
pub struct Code<'a> {
    name: &'static str,
    code: &'a str,
}

impl<'a> Code<'a> {
    pub fn module(name: &'static str, code: &'a str) -> Code<'a> {
        Code { name, code }
    }

    pub fn script(code: &'a str) -> Code<'a> {
        Code {
            name: "script",
            code,
        }
    }
}

pub fn build(source: Code, address: &AccountAddress, disable_std: bool) -> Result<CompiledUnit> {
    build_with_deps(source, vec![], address, disable_std)
}

pub fn build_with_deps(
    source: Code,
    deps: Vec<Code>,
    address: &AccountAddress,
    disable_std: bool,
) -> Result<CompiledUnit> {
    let address = Address::try_from(address.as_ref()).map_err(Error::msg)?;
    let pprog_res = parse_program(&source, &deps, disable_std)?;
    let mut prog = compile_program(pprog_res, Some(address)).map_err(|errs| {
        let mut sources = HashMap::new();
        sources.insert(source.name, source.code.to_owned());
        for dep in deps {
            sources.insert(dep.name, dep.code.to_owned());
        }

        match String::from_utf8(report_errors_to_buffer(sources, errs)) {
            Ok(error) => Error::msg(error),
            Err(err) => Error::new(err),
        }
    })?;
    Ok(prog.remove(0))
}

pub fn parse_module(
    src: &str,
    name: &'static str,
) -> Result<(Option<parser::ast::FileDefinition>, Errors)> {
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

fn parse_program(
    source: &Code,
    deps: &[Code],
    disable_std: bool,
) -> Result<Result<parser::ast::Program, Errors>> {
    let mut source_definitions = Vec::new();
    let mut lib_definitions = Vec::new();
    let mut errors: Errors = Vec::new();

    let (def_opt, mut es) = parse_module(source.code, source.name)?;
    if let Some(def) = def_opt {
        source_definitions.push(def);
    }
    errors.append(&mut es);

    for dep in deps {
        let (def_opt, mut es) = parse_module(dep.code, dep.name)?;
        if let Some(def) = def_opt {
            lib_definitions.push(def);
        }
        errors.append(&mut es);
    }

    if !disable_std {
        for module in stdlib() {
            let (def_opt, _) = parse_module(&module, "std")?;
            if let Some(def) = def_opt {
                lib_definitions.push(def);
            }
        }
    }

    let res = if errors.is_empty() {
        Ok(parser::ast::Program {
            source_definitions,
            lib_definitions,
        })
    } else {
        Err(errors)
    };
    Ok(res)
}

#[cfg(test)]
mod test {
    use libra_types::account_address::AccountAddress;
    use vm::access::{ModuleAccess, ScriptAccess};
    use vm::CompiledModule;

    pub fn compile_module(
        source: &str,
        lang: Lang,
        sender_address: &AccountAddress,
    ) -> CompiledModule {
        let compiler = lang.compiler();
        build_std_with_compiler(
            Stdlib {
                modules: mvir_std(),
                lang,
            },
            compiler.as_ref(),
        )
        .unwrap();

        CompiledModule::deserialize(&compiler.build_module(source, sender_address, true).unwrap())
            .unwrap()
    }

    use crate::vm::compiler::{
        mv::{build_with_deps, Code, build},
        Lang,
    };
    use crate::vm::stdlib::{build_std_with_compiler, Stdlib, mvir_std};
    use crate::vm::compile_script;

    #[test]
    pub fn test_build_module_success() {
        let program = "module M {}";
        build(Code::module("M", program), &AccountAddress::random(), false)
            .unwrap()
            .serialize();
    }

    #[test]
    pub fn test_build_module_failed() {
        let program = "module M {";
        let error = build(Code::module("M", program), &AccountAddress::random(), false)
            .err()
            .unwrap();
        assert!(error.to_string().contains("Unexpected end-of-file"));
    }

    #[test]
    pub fn test_build_script() {
        let program = "fun main() {}";
        build(Code::script(program), &AccountAddress::random(), false)
            .unwrap()
            .serialize();
    }

    #[test]
    pub fn test_build_script_with_dependence() {
        let dep = "\
        address 0x1:
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
        build_with_deps(
            Code::script(program),
            vec![Code::module("M", dep)],
            &AccountAddress::random(),
            false,
        )
        .unwrap()
        .serialize();
    }

    #[test]
    fn test_parse_mvir_script_with_bech32_addresses() {
        let program = r"
            import cosmos1sxqtxa3m0nh5fu2zkyfvh05tll8fmz8tk2e22e.WingsAccount;
            main() {
                return;
            }
        ";

        let script = compile_script(program, Lang::MvIr, &AccountAddress::default());

        let module = script
            .module_handles()
            .iter()
            .find(|h| script.identifier_at(h.name).to_string() == "WingsAccount")
            .unwrap();
        let address = script.address_at(module.address);
        assert_eq!(
            address.to_string(),
            "636f736d6f730000000000008180b3763b7cef44f142b112cbbe8bffce9d88eb"
        );
    }

    #[test]
    fn test_parse_mvir_module_with_bech32_addresses() {
        let program = r"
            module M {
                import cosmos1sxqtxa3m0nh5fu2zkyfvh05tll8fmz8tk2e22e.WingsAccount;
            }
        ";

        let main_module = compile_module(program, Lang::MvIr, &AccountAddress::default());

        let module = main_module
            .module_handles()
            .iter()
            .find(|h| main_module.identifier_at(h.name).to_string() == "WingsAccount")
            .unwrap();
        let address = main_module.address_at(module.address);
        assert_eq!(
            address.to_string(),
            "636f736d6f730000000000008180b3763b7cef44f142b112cbbe8bffce9d88eb"
        );
    }
}
