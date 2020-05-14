use std::path::{Path, PathBuf};
use crate::manifest::CmoveToml;
use std::fs;
use walkdir::WalkDir;
use libra::move_lang;
use std::fs::{File, OpenOptions};
use crate::compiler::bech32::bech32_into_libra;
use std::io::Write;
use crate::compiler::preprocessor;
use anyhow::Result;
use move_lang::shared::Address;
use move_lang::errors::FilesSourceText;
use move_lang::compiled_unit::CompiledUnit;
use move_lang::{compiled_unit, errors};

pub struct Builder<'a> {
    project_dir: &'a Path,
    manifest: CmoveToml,
}

impl<'a> Builder<'a> {
    pub fn new(project_dir: &'a Path, manifest: CmoveToml) -> Builder<'a> {
        Builder { project_dir, manifest }
    }

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

        let modules_output = self.modules_out_dir()?;
        if !modules_output.exists() {
            fs::create_dir_all(&modules_output)?;
        }

        let scripts_output = self.scripts_out_dir()?;
        if !scripts_output.exists() {
            fs::create_dir_all(&scripts_output)?;
        }

        let deps_dir = self.deps_dir()?;
        if !deps_dir.exists() {
            fs::create_dir_all(&deps_dir)?;
        }

        Ok(())
    }

    pub fn load_dependencies(&self, sources: &[PathBuf]) -> Result<Vec<PathBuf>> {
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
        add_source(&mut source_list, &self.source_scripts_dir()?);

        Ok(source_list)
    }

    pub fn preprocess_source_map(&self, source_map: Vec<PathBuf>) -> Result<Vec<PathBuf>> {
        let temp_src = self.temp_dir()?.join("src");
        if !temp_src.exists() {
            fs::create_dir_all(&temp_src)?;
        }

        let module_source = self.source_modules_dir()?;
        let scripts_source = self.source_scripts_dir()?;

        let temp_modules = temp_src.join("modules");
        let temp_scripts = temp_src.join("scripts");
        let mut sources = Vec::with_capacity(source_map.len());
        for src in source_map {
            let new_path = if src.starts_with(&module_source) {
                let path = src.strip_prefix(&module_source)?;
                let new_path = if let Some(parent) = path.parent() {
                    temp_modules.join(parent)
                } else {
                    temp_modules.to_owned()
                };
                fs::create_dir_all(&new_path)?;
                new_path.join(
                    path.file_name()
                        .ok_or_else(|| anyhow!("Expected file name."))?,
                )
            } else {
                let path = src.strip_prefix(&scripts_source)?;
                let new_path = if let Some(parent) = path.parent() {
                    temp_scripts.join(parent)
                } else {
                    temp_scripts.to_owned()
                };
                fs::create_dir_all(&new_path)?;
                new_path.join(
                    path.file_name()
                        .ok_or_else(|| anyhow!("Expected file name."))?,
                )
            };

            let source = preprocessor::pre_processing(&fs::read_to_string(&src)?);
            let mut f = OpenOptions::new()
                .create(true)
                .write(true)
                .open(&new_path)?;
            f.write_all(source.as_bytes())?;
            sources.push(new_path);
        }
        Ok(sources)
    }

    pub fn compile(
        &self,
        source_list: Vec<PathBuf>,
        dep_list: Vec<PathBuf>,
    ) -> Result<(FilesSourceText, Vec<CompiledUnit>)> {
        let source_list = Self::convert_path(source_list)?;
        let dep_list = Self::convert_path(dep_list)?;
        let addr = self.address()?;
        Ok(move_lang::move_compile(&source_list, &dep_list, addr)?)
    }

    pub fn check(&self, source_list: Vec<PathBuf>, dep_list: Vec<PathBuf>) -> Result<()> {
        let source_list = Self::convert_path(source_list)?;
        let dep_list = Self::convert_path(dep_list)?;
        let addr = self.address()?;
        Ok(move_lang::move_check(&source_list, &dep_list, addr)?)
    }

    fn convert_path(path_list: Vec<PathBuf>) -> Result<Vec<String>> {
        path_list
            .iter()
            .map(|path| path.to_str().map(|path| path.to_owned()))
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| anyhow!("Failed to convert source path"))
    }

    pub fn verify_and_store(&self, files: FilesSourceText, compiled_units: Vec<CompiledUnit>) -> Result<()> {
        let (compiled_units, ice_errors) = compiled_unit::verify_units(compiled_units);
        let (modules, scripts): (Vec<_>, Vec<_>) = compiled_units
            .into_iter()
            .partition(|u| matches!(u, CompiledUnit::Module { .. }));

        fn store(units: Vec<CompiledUnit>, base_dir: &PathBuf) -> Result<()> {
            for (idx, unit) in units.into_iter().enumerate() {
                let mut path = base_dir.join(format!("{}_{}", idx, unit.name()));
                path.set_extension("mv");
                File::create(&path)?.write_all(&unit.serialize())?
            }
            Ok(())
        }

        if !modules.is_empty() {
            let modules_dir = self.modules_out_dir()?;
            if modules_dir.exists() {
                fs::remove_dir_all(&modules_dir)?;
                fs::create_dir_all(&modules_dir)?;
            }
            store(modules, &modules_dir)?;
        }

        if !scripts.is_empty() {
            let scripts_dir = self.scripts_out_dir()?;
            if scripts_dir.exists() {
                fs::remove_dir_all(&scripts_dir)?;
                fs::create_dir_all(&scripts_dir)?;
            }

            store(scripts, &scripts_dir)?;
        }

        if !ice_errors.is_empty() {
            errors::report_errors(files, ice_errors);
        }
        Ok(())
    }

    fn address(&self) -> Result<Option<Address>> {
        let package = &self.manifest.package;
        match package.account_address.as_ref().map(|addr| {
            if addr.starts_with("0x") {
                Address::parse_str(&addr).map_err(|err| anyhow!(err))
            } else {
                bech32_into_libra(&addr)
                    .and_then(|addr| Address::parse_str(&addr).map_err(|err| anyhow!(err)))
            }
        }) {
            Some(r) => r.and_then(|a| Ok(Some(a))),
            None => Ok(None),
        }
    }

    fn temp_dir(&self) -> Result<PathBuf> {
        self.manifest
            .layout
            .as_ref()
            .and_then(|l| l.temp_dir.as_ref())
            .map(|t| self.project_dir.join(t))
            .ok_or_else(|| anyhow!("Expected temp_dir"))
    }

    fn deps_dir(&self) -> Result<PathBuf> {
        self.manifest
            .layout
            .as_ref()
            .and_then(|l| l.bytecode_cache.as_ref())
            .map(|t| self.project_dir.join(t))
            .ok_or_else(|| anyhow!("Expected bytecode_cache"))
    }

    fn modules_out_dir(&self) -> Result<PathBuf> {
        self.manifest
            .layout
            .as_ref()
            .and_then(|l| l.module_output.as_ref())
            .map(|t| self.project_dir.join(t))
            .ok_or_else(|| anyhow!("Expected module_output"))
    }

    fn scripts_out_dir(&self) -> Result<PathBuf> {
        self.manifest
            .layout
            .as_ref()
            .and_then(|l| l.script_output.as_ref())
            .map(|t| self.project_dir.join(t))
            .ok_or_else(|| anyhow!("Expected script_output"))
    }

    fn source_modules_dir(&self) -> Result<PathBuf> {
        self.manifest
            .layout
            .as_ref()
            .and_then(|l| l.module_dir.as_ref())
            .map(|t| self.project_dir.join(t))
            .ok_or_else(|| anyhow!("Expected module_output"))
    }

    fn source_scripts_dir(&self) -> Result<PathBuf> {
        self.manifest
            .layout
            .as_ref()
            .and_then(|l| l.script_dir.as_ref())
            .map(|t| self.project_dir.join(t))
            .ok_or_else(|| anyhow!("Expected script_output"))
    }
}

impl<'a> Drop for Builder<'a> {
    fn drop(&mut self) {
        let res = self.temp_dir()
            .and_then(|dir| {
                if dir.exists() {
                    Ok(fs::remove_dir_all(&dir)?)
                } else {
                    Ok(())
                }
            });

        if let Err(err) = res {
            println!("Failed to clean up temporary directory:{}", err);
        }
    }
}