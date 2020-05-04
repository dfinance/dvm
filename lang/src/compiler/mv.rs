use libra::libra_state_view::StateView;
use libra::move_lang::parser::ast::*;
use std::collections::HashMap;

use anyhow::Error;
use libra::move_lang::{
    parser, parser::syntax::parse_file_string, shared::Address, strip_comments_and_verify,
    compile_program,
};
use libra::move_lang::errors::{Errors, report_errors_to_buffer};
use libra::libra_types::language_storage::ModuleId;
use crate::compiler::module_loader::ModuleLoader;
use crate::compiler::name_pull::StaticHolder;
use crate::compiler::meta::extract_meta;
use crate::compiler::preprocessor::pre_processing;

pub struct Move<'a, S>
where
    S: StateView + Clone,
{
    loader: &'a ModuleLoader<S>,
    static_holder: StaticHolder,
}

impl<'a, S> Move<'a, S>
where
    S: StateView + Clone,
{
    pub fn new(loader: &'a ModuleLoader<S>) -> Move<'a, S> {
        Move {
            loader,
            static_holder: StaticHolder::new(),
        }
    }

    pub fn compile_source_map(
        &mut self,
        source_map: HashMap<&str, &str>,
        account_address: Address,
    ) -> Result<HashMap<String, Vec<u8>>, Error> {
        let mut source_map = source_map
            .into_iter()
            .map(|(k, v)| (self.static_holder.pull(k), pre_processing(v)))
            .collect::<HashMap<_, _>>();

        let pprog_res = self.parse_program(&mut source_map)?;
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
    ) -> Result<Result<Program, Errors>, Error> {
        let mut errors: Errors = Vec::new();
        let mut dep_source_map = HashMap::new();
        let mut source_definitions = Vec::with_capacity(source_map.len());
        let mut lib_definitions = HashMap::new();

        for (name, source) in source_map.iter() {
            let (def_opt, mut es) = Self::parse_module(source, name)?;
            if let Some(def) = def_opt {
                self.load_dependency(&def, &mut dep_source_map, &mut lib_definitions)?;
                source_definitions.push(def);
            }
            errors.append(&mut es);
        }

        for dep_source in dep_source_map {
            source_map.insert(dep_source.0, dep_source.1);
        }

        if errors.is_empty() {
            Ok(Ok(parser::ast::Program {
                source_definitions,
                lib_definitions: lib_definitions.into_iter().map(|(_, v)| v).collect(),
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

    fn load_dependency(
        &mut self,
        definition: &FileDefinition,
        source_map: &mut HashMap<&'static str, String>,
        definition_map: &mut HashMap<ModuleId, FileDefinition>,
    ) -> Result<(), Error> {
        let meta = extract_meta(definition)?;
        let dep_list = self.loader.load_modules_signature(&meta.dep_list)?;
        for signature in dep_list {
            let source = signature.to_string();
            let name = self
                .static_holder
                .pull(&signature.self_id().name().as_str().to_string());

            let def = self.make_file_definition(&source, name)?;
            self.load_dependency(&def, source_map, definition_map)?;

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
        let (def, err) = Self::parse_module(source, name)?;
        if !err.is_empty() {
            Err(Error::msg(format!("Failed to parse module{:?};", err)))
        } else {
            def.ok_or_else(|| Error::msg("Expected module".to_string()))
        }
    }

    pub fn error_render(&self, errors: Errors, source_map: HashMap<&'static str, String>) -> Error {
        match String::from_utf8(report_errors_to_buffer(source_map, errors)) {
            Ok(error) => Error::msg(error),
            Err(err) => Error::new(err),
        }
    }
}
