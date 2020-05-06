use libra::move_lang::parser::ast::{FileDefinition, ModuleOrAddress};
use libra::libra_types::language_storage::ModuleId;
use crate::compiler::imports::ImportsExtractor;
use std::iter::FromIterator;
use anyhow::Error;

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
pub struct ModuleMeta {
    pub module_name: String,
    pub dep_list: Vec<ModuleId>,
}

pub fn extract_meta(file_definition: &FileDefinition) -> Result<ModuleMeta, Error> {
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
