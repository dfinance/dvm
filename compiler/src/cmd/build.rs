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

    let (text_source, units) = builder.compile(pre_processed_source_map, dep_list)?;
    builder.verify_and_store(text_source, units)
}

mod test {
    use std::env;
    use crate::manifest::{CmoveToml, Layout};
    use crate::cmd::{new, build, update};

    #[test]
    fn tst() {
        let dir = env::current_dir().unwrap();
        let mut cmove = CmoveToml::default();
        let mut layout = Layout::default();
        layout.fill();
        cmove.layout = Some(layout);
        // new::execute(&dir, "test".to_string(), None, None).unwrap();
        build::execute(&dir.join("test"), cmove.clone()).unwrap();
        update::execute(&dir.join("test"), cmove).unwrap();
    }
}
