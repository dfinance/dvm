use std::path::{PathBuf, Path};
use anyhow::Result;
use libra::libra_types::language_storage::ModuleId;
use tiny_keccak::{Hasher, Sha3};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use http::Uri;
use crate::manifest::CmoveToml;
use std::fs;

pub trait BytecodeSource: Clone {
    fn load(&self, module_id: &ModuleId) -> Result<Vec<u8>>;
}

#[derive(Clone)]
pub struct ZeroSource;

impl BytecodeSource for ZeroSource {
    fn load(&self, module_id: &ModuleId) -> Result<Vec<u8>> {
        Err(anyhow!("Module {:?} not found", module_id))
    }
}

#[derive(Clone)]
pub struct RestBytecodeSource {
    url: Uri,
}

impl RestBytecodeSource {
    pub fn new(url: Uri) -> RestBytecodeSource {
        RestBytecodeSource { url }
    }
}

impl BytecodeSource for RestBytecodeSource {
    fn load(&self, _module_id: &ModuleId) -> Result<Vec<u8>> {
        todo!("Load dependencies from node REST API.")
    }
}

#[derive(Clone)]
pub struct Loader<S: BytecodeSource> {
    cache_path: Option<PathBuf>,
    source: S,
}

impl<S> Loader<S>
where
    S: BytecodeSource,
{
    pub fn new(cache_path: Option<PathBuf>, source: S) -> Loader<S> {
        Loader { cache_path, source }
    }

    pub fn get(&self, module_id: &ModuleId) -> Result<Vec<u8>> {
        let name = self.make_local_name(&module_id)?;

        if let Some(cache_path) = &self.cache_path {
            let local_path = cache_path.join(name);
            if local_path.exists() {
                let mut f = File::open(local_path)?;
                let mut bytecode = Vec::new();
                f.read_to_end(&mut bytecode)?;
                Ok(bytecode)
            } else {
                let bytecode = self.source.load(module_id)?;
                let mut f = OpenOptions::new()
                    .create(true)
                    .write(true)
                    .open(&local_path)?;
                f.write_all(&bytecode)?;
                Ok(bytecode)
            }
        } else {
            self.source.load(module_id)
        }
    }

    fn make_local_name(&self, module_id: &ModuleId) -> Result<String> {
        let mut digest = Sha3::v256();
        digest.update(module_id.name().as_bytes());
        digest.update(module_id.address().as_ref());
        let mut output = [0; 32];
        digest.finalize(&mut output);
        Ok(hex::encode(&output))
    }
}

pub fn make_rest_loader(
    project_dir: &Path,
    cmove: &CmoveToml,
) -> Result<Option<Loader<RestBytecodeSource>>> {
    let path = cmove
        .layout
        .as_ref()
        .and_then(|l| l.bytecode_cache.as_ref())
        .ok_or_else(|| anyhow!("Expected cache path"))?;
    let cache_path = project_dir.join(path);
    if !cache_path.exists() {
        fs::create_dir_all(&cache_path)?;
    }

    Ok(if let Some(uri) = cmove.package.blockchain_api.as_ref() {
        Some(Loader::new(
            Some(cache_path),
            RestBytecodeSource::new(uri.parse()?),
        ))
    } else {
        None
    })
}
