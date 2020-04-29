use libra::libra_state_view::StateView;
use libra::libra_types::account_address::AccountAddress;
use anyhow::Error;
use libra::libra_types::language_storage::ModuleId;
use libra::move_ir_types::ast::ModuleIdent;
use libra::compiler::Compiler as MvIrCompiler;
use libra::bytecode_verifier::{VerifiedModule, VerifiedScript};
use libra::ir_to_bytecode;
use crate::compiler::module_loader::ModuleLoader;
use crate::compiler::{ModuleMeta, Builder, replace_u_literal};
use crate::bech32::replace_bech32_addresses;
use libra::move_core_types::identifier::Identifier;

#[derive(Clone)]
pub struct Mvir<S>
where
    S: StateView + Clone,
{
    loader: ModuleLoader<S>,
}

impl<S> Mvir<S>
where
    S: StateView + Clone,
{
    pub fn new(module_loader: ModuleLoader<S>) -> Mvir<S> {
        Mvir {
            loader: module_loader,
        }
    }

    fn extract_meta(code: &str, is_module: bool) -> Result<ModuleMeta, Error> {
        let (name, imports) = if is_module {
            let module = ir_to_bytecode::parser::parse_module("module", code)?;
            (module.name.to_string(), module.imports)
        } else {
            let script = ir_to_bytecode::parser::parse_script("script", code)?;
            ("main".to_owned(), script.imports)
        };

        let mut imported_modules = Vec::with_capacity(imports.len());
        for import in imports {
            if let ModuleIdent::Qualified(module_ident) = import.ident {
                imported_modules.push(ModuleId::new(
                    module_ident.address,
                    Identifier::new(module_ident.name.into_inner())?,
                ));
            }
        }
        Ok(ModuleMeta {
            module_name: name,
            dep_list: imported_modules,
        })
    }
}

impl<S> Builder for Mvir<S>
where
    S: StateView + Clone,
{
    fn build_module(&self, code: &str, address: &AccountAddress) -> Result<Vec<u8>, Error> {
        let code = pre_processing(code);
        let dep_list = self
            .loader
            .load_verified_modules(&Self::extract_meta(&code, true)?.dep_list)?;

        let mut compiler = MvIrCompiler::default();
        compiler.skip_stdlib_deps = true;
        compiler.extra_deps = dep_list;
        compiler.address = *address;

        let module = compiler.into_compiled_module("module", &code)?;
        let module = VerifiedModule::new(module)
            .map_err(|(_, err)| Error::msg(format!("Verification error:{:?}", err)))?;

        let mut buff = Vec::new();
        module.serialize(&mut buff).unwrap();
        Ok(buff)
    }

    fn build_script(&self, code: &str, address: &AccountAddress) -> Result<Vec<u8>, Error> {
        let code = pre_processing(code);
        let dep_list = self
            .loader
            .load_verified_modules(&Self::extract_meta(&code, false)?.dep_list)?;

        let mut compiler = MvIrCompiler::default();
        compiler.skip_stdlib_deps = true;
        compiler.extra_deps = dep_list;
        compiler.address = *address;

        let (program, _) = compiler.into_compiled_script_and_source_map("script", &code)?;

        let program = VerifiedScript::new(program)
            .map_err(|err| Error::msg(format!("Verification error:{:?}", err)))?;

        let mut buff = Vec::new();
        program.serialize(&mut buff)?;
        Ok(buff)
    }

    fn module_meta(&self, code: &str) -> Result<ModuleMeta, Error> {
        let code = pre_processing(&code);
        Self::extract_meta(&code, code.contains("module"))
    }
}

fn pre_processing(code: &str) -> String {
    let code = replace_bech32_addresses(code);
    replace_u_literal(&code)
}
