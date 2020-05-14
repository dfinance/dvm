use anyhow::Result;
use std::path::Path;
use crate::manifest::CmoveToml;
use crate::compiler::builder::Builder;

pub fn execute(project_dir: &Path, manifest: CmoveToml) -> Result<()> {
    let builder = Builder::new(project_dir, manifest);
    builder.init_build_layout()?;

    let source_map = builder.make_source_map()?;
    let pre_processed_source_map = builder.preprocess_source_map(source_map)?;

    let bytecode_list = builder.load_dependencies(&pre_processed_source_map)?;
    let dep_list = builder.make_dependencies_as_source(bytecode_list)?;

    builder.check(pre_processed_source_map, dep_list)
}
