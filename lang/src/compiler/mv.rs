use libra::libra_state_view::StateView;
use libra::move_lang::parser::ast::*;
use std::collections::{HashMap, HashSet};

use anyhow::Error;
use libra::move_lang::{
    parser, parser::syntax::parse_file_string, shared::Address, strip_comments_and_verify,
    compile_program,
};
use libra::move_lang::errors::{Errors, report_errors_to_buffer};
use libra::libra_types::language_storage::ModuleId;
use crate::compiler::module_loader::ModuleLoader;
use crate::compiler::name_pull::StrTable;
use crate::compiler::meta::{extract_meta, ModuleMeta};
use crate::compiler::preprocessor::pre_processing;
use libra::libra_types::account_address::AccountAddress;
use std::convert::TryFrom;
use libra::move_core_types::identifier::Identifier;

pub struct Move<'a, S>
where
    S: StateView + Clone,
{
    loader: &'a ModuleLoader<S>,
    static_holder: StrTable,
}

impl<'a, S> Move<'a, S>
where
    S: StateView + Clone,
{
    pub fn new(loader: &'a ModuleLoader<S>) -> Move<'a, S> {
        Move {
            loader,
            static_holder: StrTable::new(),
        }
    }

    pub fn compile_source_map(
        &mut self,
        source_map: HashMap<String, String>,
        account_address: AccountAddress,
    ) -> Result<HashMap<String, Vec<u8>>, Error> {
        let mut source_map = source_map
            .into_iter()
            .map(|(k, v)| (self.static_holder.pull(k), pre_processing(&v)))
            .collect::<HashMap<_, _>>();

        let pprog_res = self.parse_program(&mut source_map, &account_address)?;

        let account_address = Address::try_from(account_address.as_ref()).map_err(Error::msg)?;
        let prog = compile_program(pprog_res, Some(account_address))
            .map_err(|errs| self.error_render(errs, source_map))?;

        Ok(prog
            .into_iter()
            .map(|unit| (unit.name(), unit.serialize()))
            .collect())
    }

    fn parse_program(
        &mut self,
        source_map: &mut HashMap<&'static str, String>,
        account_address: &AccountAddress,
    ) -> Result<Result<Program, Errors>, Error> {
        let mut errors: Errors = Vec::new();
        let mut source_definitions = Vec::with_capacity(source_map.len());
        let mut source_ids = HashSet::with_capacity(source_map.len());

        for (name, source) in source_map.iter() {
            let (def_opt, mut es) = Self::parse_module(source, name)?;
            if let Some(def) = def_opt {
                let meta = extract_meta(&def)?;
                source_ids.insert(ModuleId::new(
                    *account_address,
                    Identifier::new(meta.module_name.to_owned()).unwrap(),
                ));
                source_definitions.push((def, meta));
            }
            errors.append(&mut es);
        }
        let mut dep_loader = DepLoader::new(self.loader, source_ids, &mut self.static_holder);
        for (_, meta) in &source_definitions {
            dep_loader.load(meta)?;
        }

        let (lib_source_map, lib_definitions) = dep_loader.results();
        source_map.extend(lib_source_map);

        if errors.is_empty() {
            Ok(Ok(parser::ast::Program {
                source_definitions: source_definitions.into_iter().map(|def| def.0).collect(),
                lib_definitions,
            }))
        } else {
            Ok(Err(errors))
        }
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

    pub fn error_render(&self, errors: Errors, source_map: HashMap<&'static str, String>) -> Error {
        match String::from_utf8(report_errors_to_buffer(source_map, errors)) {
            Ok(error) => Error::msg(error),
            Err(err) => Error::new(err),
        }
    }
}

struct DepLoader<'a, S>
where
    S: StateView + Clone,
{
    lib_source_map: HashMap<&'static str, String>,
    lib_definition: HashMap<ModuleId, FileDefinition>,
    loader: &'a ModuleLoader<S>,
    source_ids: HashSet<ModuleId>,
    static_holder: &'a mut StrTable,
}

impl<'a, S> DepLoader<'a, S>
where
    S: StateView + Clone,
{
    pub fn new(
        loader: &'a ModuleLoader<S>,
        source_ids: HashSet<ModuleId>,
        static_holder: &'a mut StrTable,
    ) -> DepLoader<'a, S> {
        DepLoader {
            lib_source_map: Default::default(),
            lib_definition: Default::default(),
            loader,
            source_ids,
            static_holder,
        }
    }

    pub fn load(&mut self, meta: &ModuleMeta) -> Result<(), Error> {
        let dep_list = self
            .loader
            .load_modules_signature(&self.filter_dep_list(&meta.dep_list))?;
        for signature in dep_list {
            let source = signature.to_string();
            let name = self.static_holder.pull(format!(
                "{}{}",
                hex::encode(signature.self_id().address().as_ref()),
                signature.self_id().name().as_str()
            ));

            let def = self.make_file_definition(&source, name)?;
            let meta = extract_meta(&def)?;
            self.load(&meta)?;

            self.lib_source_map.insert(name, source);
            self.lib_definition.insert(signature.self_id().clone(), def);
        }
        Ok(())
    }

    fn filter_dep_list(&self, dep_list: &[ModuleId]) -> Vec<ModuleId> {
        dep_list
            .iter()
            .filter(|dep| !self.source_ids.contains(dep))
            .map(|dep| dep.to_owned())
            .collect()
    }

    fn make_file_definition(
        &self,
        source: &str,
        name: &'static str,
    ) -> Result<FileDefinition, Error> {
        let (def, err) = Move::<S>::parse_module(source, name)?;
        if !err.is_empty() {
            Err(Error::msg(format!("Failed to parse module{:?};", err)))
        } else {
            def.ok_or_else(|| Error::msg("Expected module".to_string()))
        }
    }

    pub fn results(self) -> (HashMap<&'static str, String>, Vec<FileDefinition>) {
        (
            self.lib_source_map,
            self.lib_definition.into_iter().map(|(_, v)| v).collect(),
        )
    }
}
