use anyhow::Result;
use std::path::Path;
use crate::manifest::MoveToml;
use crate::mv::builder::Builder;
use crate::mv::dependence::loader::make_rest_loader;

pub fn execute(project_dir: &Path, manifest: MoveToml) -> Result<()> {
    let loader = make_rest_loader(&project_dir, &manifest)?;
    let builder = Builder::new(project_dir, manifest, &loader, true, true);
    builder.init_build_layout()?;

    let source_map = builder.make_source_map()?;
    let pre_processed_source_map = builder.preprocess_source_map(source_map)?;

    let bytecode_map = builder.load_dependencies(&pre_processed_source_map)?;
    let dep_list = builder.make_dependencies_as_source(bytecode_map)?;

    let (text_source, units) = builder.compile(pre_processed_source_map, dep_list)?;
    builder.verify_and_store(text_source, units)
}
