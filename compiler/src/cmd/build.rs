use anyhow::Error;
use anyhow::Result;
use std::path::{Path, PathBuf};
use crate::manifest::CmoveToml;
use std::fs;
use walkdir::{WalkDir, DirEntry};
use crate::compiler::preprocessor;
use std::fs::OpenOptions;
use std::io::Write;

pub fn execute(project_dir: &Path, manifest: CmoveToml) -> Result<()> {
    let builder = Builder { project_dir, manifest };
    builder.init_build_layout()?;

    let bytecode_list = builder.load_dependencies()?;
    let dep_list = builder.make_dependencies_as_source(bytecode_list)?;
    let source_map = builder.make_source_map()?;
    let pre_processed_source_map = builder.preprocess_source_map(source_map)?;


    todo!()
}

struct Builder<'a> {
    project_dir: &'a Path,
    manifest: CmoveToml,
}

impl<'a> Builder<'a> {
    pub fn init_build_layout(&self) -> Result<()> {
        let temp_dir = self.temp_dir()?;
        if temp_dir.exists() {
            fs::remove_dir_all(&temp_dir)?;
        }
        fs::create_dir_all(&temp_dir)?;

        let deps_dir = self.deps_dir()?;
        if !temp_dir.exists() {
            fs::create_dir_all(&deps_dir)?;
        }

        let modules_output = self.modules_dir()?;
        if !modules_output.exists() {
            fs::create_dir_all(&modules_output)?;
        }

        let scripts_output = self.scripts_dir()?;
        if !scripts_output.exists() {
            fs::create_dir_all(&scripts_output)?;
        }

        let deps_dir = self.deps_dir()?;
        if !deps_dir.exists() {
            fs::create_dir_all(&deps_dir)?;
        }

        Ok(())
    }

    pub fn load_dependencies(&self) -> Result<Vec<PathBuf>> {
        //todo
        Ok(vec![])
    }

    pub fn make_dependencies_as_source(&self, bytecode: Vec<PathBuf>) -> Result<Vec<PathBuf>> {
        Ok(vec![])
    }

    pub fn make_source_map(&self) -> Result<Vec<PathBuf>> {
        fn add_source(sources: &mut Vec<PathBuf>, path: &Path) {
            for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
                let path = entry.into_path();
                if let Some(extension) = path.extension() {
                    if extension == "move" {
                        sources.push(path.to_owned());
                    }
                }
            }
        }

        let mut source_list = vec![];
        add_source(&mut source_list, &self.source_modules_dir()?);
        add_source(&mut source_list, &self.source_modules_dir()?);

        Ok(source_list)
    }

    pub fn preprocess_source_map(&self, source_map: Vec<PathBuf>) -> Result<Vec<PathBuf>> {
        let temp_src = self.temp_dir()?.join("src");
        if !temp_src.exists() {
            fs::create_dir_all(&temp_src)?;
        }

        //temp_src.

        let mut sources = Vec::with_capacity(source_map.len());
        for src in source_map {
            let mut source = preprocessor::pre_processing(&fs::read_to_string(&src)?);
            let temp_name = temp_src.join(src.file_name().ok_or_else(|| anyhow!("Expected source file name."))?);

            let mut f = OpenOptions::new()
                .create(true)
                .write(true)
                .open(&temp_name)?;
            f.write_all(source.as_bytes())?;
            sources.push(temp_name);
        }
        Ok(sources)
    }

    pub fn compile(&self) -> Result<()> {
        todo!()
    }

    fn temp_dir(&self) -> Result<PathBuf> {
        self.manifest.layout.as_ref()
            .and_then(|l| l.temp_dir.as_ref())
            .map(|t| self.project_dir.join(t))
            .ok_or_else(|| anyhow!("Expected temp_dir"))
    }

    fn deps_dir(&self) -> Result<PathBuf> {
        self.manifest.layout.as_ref()
            .and_then(|l| l.bytecode_cache.as_ref())
            .map(|t| self.project_dir.join(t))
            .ok_or_else(|| anyhow!("Expected bytecode_cache"))
    }

    fn modules_dir(&self) -> Result<PathBuf> {
        self.manifest.layout.as_ref()
            .and_then(|l| l.module_output.as_ref())
            .map(|t| self.project_dir.join(t))
            .ok_or_else(|| anyhow!("Expected module_output"))
    }

    fn scripts_dir(&self) -> Result<PathBuf> {
        self.manifest.layout.as_ref()
            .and_then(|l| l.script_output.as_ref())
            .map(|t| self.project_dir.join(t))
            .ok_or_else(|| anyhow!("Expected script_output"))
    }

    fn source_modules_dir(&self) -> Result<PathBuf> {
        self.manifest.layout.as_ref()
            .and_then(|l| l.module_dir.as_ref())
            .map(|t| self.project_dir.join(t))
            .ok_or_else(|| anyhow!("Expected module_output"))
    }

    fn source_scripts_dir(&self) -> Result<PathBuf> {
        self.manifest.layout.as_ref()
            .and_then(|l| l.script_dir.as_ref())
            .map(|t| self.project_dir.join(t))
            .ok_or_else(|| anyhow!("Expected script_output"))
    }
}

mod test {
    use std::env;
    use crate::manifest::{CmoveToml, Layout};
    use crate::cmd::{new, build};

    #[test]
    fn tst() {
        let dir = env::current_dir().unwrap();
        let mut cmove = CmoveToml::default();
        let mut layout = Layout::default();
        layout.fill();
        cmove.layout = Some(layout);
        // new::execute(&dir, "test".to_string(), None, None).unwrap();
        build::execute(&dir.join("test"), cmove).unwrap();
    }
}