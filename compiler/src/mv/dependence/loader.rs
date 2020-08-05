use std::path::{PathBuf, Path};
use anyhow::Result;

use libra::prelude::*;

use tiny_keccak::{Hasher, Sha3};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use http::Uri;
use crate::manifest::MoveToml;
use std::fs;
use serde::{Deserialize, Serialize};

/// Module loader.
pub trait BytecodeLoader: Clone {
    /// Loads module bytecode by it module id.
    fn load(&self, module_id: &ModuleId) -> Result<Vec<u8>>;
}

/// Empty module loader.
/// Mock.
#[derive(Clone)]
pub struct ZeroLoader;

impl BytecodeLoader for ZeroLoader {
    fn load(&self, module_id: &ModuleId) -> Result<Vec<u8>> {
        Err(anyhow!("Module {:?} not found", module_id))
    }
}

/// Bytecode loader which loads bytecode by dnode REST api.
#[derive(Clone)]
pub struct RestBytecodeLoader {
    url: Uri,
}

impl RestBytecodeLoader {
    /// Create a new `RestBytecodeLoader` with dnode api base url.
    pub fn new(url: Uri) -> RestBytecodeLoader {
        RestBytecodeLoader { url }
    }
}

impl BytecodeLoader for RestBytecodeLoader {
    fn load(&self, module_id: &ModuleId) -> Result<Vec<u8>> {
        let path = AccessPath::code_access_path(module_id);
        let url = format!(
            "{base_url}vm/data/{address}/{path}",
            base_url = self.url,
            address = hex::encode(&path.address),
            path = hex::encode(path.path)
        );

        let resp = reqwest::blocking::get(&url)?;
        if resp.status().is_success() {
            let res: LoaderResponse = resp.json()?;
            Ok(hex::decode(&res.result.value)?)
        } else {
            let res: LoaderErrorResponse = resp.json()?;
            Err(anyhow!(
                "Failed to load dependencies :'{}' [{}]",
                url,
                res.error
            ))
        }
    }
}

/// Api response.
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct LoaderResponse {
    /// Result.
    result: Response,
}

/// Success response.
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Response {
    /// Hex encoded bytecode.
    value: String,
}

///Error response.
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct LoaderErrorResponse {
    error: String,
}

/// Module loader.
#[derive(Clone)]
pub struct Loader<S: BytecodeLoader> {
    cache_path: Option<PathBuf>,
    source: S,
}

impl<S> Loader<S>
where
    S: BytecodeLoader,
{
    /// Create a new module loader with cache path and external module source.
    pub fn new(cache_path: Option<PathBuf>, source: S) -> Loader<S> {
        Loader { cache_path, source }
    }

    /// Loads module by module id.
    /// Tries to load the module from the local cache.
    ///  Then tries to load the module from the external module source if the module doesn't exist in cache.
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

/// Makes a RestBytecodeLoader with project path and project manifest.
pub fn make_rest_loader(
    project_dir: &Path,
    cmove: &MoveToml,
) -> Result<Option<Loader<RestBytecodeLoader>>> {
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
            RestBytecodeLoader::new(uri.parse()?),
        ))
    } else {
        None
    })
}
