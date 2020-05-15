use serde_derive::{Serialize, Deserialize};
use anyhow::Error;
use std::path::Path;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use toml::Value;

pub const MANIFEST: &str = "Move.toml";

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct MoveToml {
    pub package: Package,
    pub layout: Option<Layout>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Package {
    pub name: Option<String>,
    pub account_address: Option<String>,
    pub authors: Option<Vec<String>>,
    pub blockchain_api: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Layout {
    pub module_dir: Option<String>,
    pub script_dir: Option<String>,
    pub bytecode_cache: Option<String>,
    pub module_output: Option<String>,
    pub script_output: Option<String>,
    pub temp_dir: Option<String>,
}

impl Layout {
    pub fn new() -> Layout {
        Layout {
            module_dir: None,
            script_dir: None,
            bytecode_cache: None,
            module_output: None,
            script_output: None,
            temp_dir: None,
        }
    }

    pub fn fill(&mut self) {
        self.module_dir
            .get_or_insert_with(|| "src/modules".to_owned());
        self.script_dir
            .get_or_insert_with(|| "src/scripts".to_owned());
        self.bytecode_cache
            .get_or_insert_with(|| "target/deps".to_owned());
        self.module_output
            .get_or_insert_with(|| "target/artifacts/modules".to_owned());
        self.script_output
            .get_or_insert_with(|| "target/artifacts/scripts".to_owned());
        self.temp_dir
            .get_or_insert_with(|| "target/build".to_owned());
    }
}

pub fn read_manifest(path: &Path) -> Result<MoveToml, Error> {
    Ok(toml::from_str(&fs::read_to_string(path)?)?)
}

pub fn store_manifest(path: &Path, manifest: MoveToml) -> Result<(), Error> {
    let value = toml::to_vec(&Value::try_from(manifest)?)?;
    let mut f = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(path)?;
    f.set_len(0)?;
    f.write_all(&value)?;
    Ok(())
}
