pub mod ds_loader;

pub use libra::prelude::*;
use crate::mv::dependence::loader::Loader;
use crate::embedded::ds_loader::StateViewLoader;
use std::collections::HashMap;
use anyhow::Result;
use std::{env, fs};
use std::path::{PathBuf, Path};
use rand::Rng;
use crate::mv::builder::Builder;
use crate::manifest::{MoveToml, Layout};
use std::fs::OpenOptions;
use std::io::Write;

/// Embedded move compiler.
#[derive(Clone)]
pub struct Compiler<S: StateView + Clone> {
    loader: Option<Loader<StateViewLoader<S>>>,
}

impl<S> Compiler<S>
where
    S: StateView + Clone,
{
    /// Create move compiler.
    pub fn new(view: S) -> Compiler<S> {
        Compiler {
            loader: Some(Loader::new(None, StateViewLoader::new(view))),
        }
    }

    /// Compile multiple sources.
    pub fn compile_source_map(
        &self,
        source_map: HashMap<String, String>,
        address: Option<AccountAddress>,
    ) -> Result<HashMap<String, Vec<u8>>> {
        let dir = TempDir::new()?;
        let mut cmove = MoveToml::default();
        let mut layout = Layout::default();
        layout.fill();

        let module_dir = dir.path.join(
            layout
                .module_dir
                .as_ref()
                .ok_or_else(|| anyhow!("Expected module_dir in layout"))?,
        );
        if !module_dir.exists() {
            fs::create_dir_all(&module_dir)?;
        }

        for (name, source) in source_map {
            let mut source_path = module_dir.join(name);
            source_path.set_extension("move");
            let mut f = OpenOptions::new()
                .create(true)
                .write(true)
                .open(&module_dir.join(source_path))?;
            f.write_all(source.as_bytes())?;
        }
        cmove.package.account_address = address.map(|addr| format!("0x{}", addr));
        cmove.layout = Some(layout);

        let builder = Builder::new(dir.path(), cmove, &self.loader, false, false);
        builder.init_build_layout()?;
        let source_map = builder.preprocess_source_map(builder.make_source_map()?)?;
        let dep_list =
            builder.make_dependencies_as_source(builder.load_dependencies(&source_map)?)?;
        let (text_source, units) = builder.compile(source_map, dep_list)?;
        builder.verify(text_source, units)
    }

    /// Compiler source codes.
    pub fn compile(&self, code: &str, address: Option<AccountAddress>) -> Result<Vec<u8>> {
        let mut source_map = HashMap::new();
        source_map.insert("source".to_string(), code.to_string());
        let bytecode_map = self.compile_source_map(source_map, address)?;
        bytecode_map
            .into_iter()
            .next()
            .map(|(_, bytecode)| bytecode)
            .ok_or_else(|| anyhow!("Expected source map is not empty."))
    }
}

/// Temp directory.
/// Random temporary directory which will be removed when 'TempDir' drop.
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    /// Create a new temporary directory.
    pub fn new() -> Result<TempDir> {
        let dir = env::temp_dir();
        let mut rng = rand::thread_rng();

        let path = dir.join(format!("{}", rng.gen::<u128>()));
        if !path.exists() {
            fs::create_dir_all(&path)?;
        }

        Ok(TempDir { path })
    }

    /// Returns the directory path.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        if self.path.exists() {
            if let Err(err) = fs::remove_dir_all(&self.path) {
                error!(
                    "Failed to cline up the temporary directory '{:?}' {:?}",
                    self.path, err
                );
            }
        }
    }
}

/// Compiler string with move source code.
pub fn compile(code: &str, address: Option<AccountAddress>) -> Result<Vec<u8>> {
    let compiler = Compiler::new(ZeroStateView);
    compiler.compile(code, address)
}

/// State view mock.
#[derive(Clone)]
struct ZeroStateView;

impl StateView for ZeroStateView {
    fn get(&self, _: &AccessPath) -> Result<Option<Vec<u8>>> {
        Ok(None)
    }

    fn multi_get(&self, _: &[AccessPath]) -> Result<Vec<Option<Vec<u8>>>> {
        Ok(vec![])
    }

    fn is_genesis(&self) -> bool {
        false
    }
}
