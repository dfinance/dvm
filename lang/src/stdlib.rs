use libra::libra_types::write_set::{WriteSet, WriteOp};
use anyhow::Error;
use libra::libra_types::account_address::AccountAddress;
use libra::libra_vm::CompiledModule;
use libra::libra_types::language_storage::ModuleId;
use libra::lcs;
use serde::{Deserialize, Serialize};
use libra::bytecode_verifier::VerifiedModule;
use libra::libra_state_view::StateView;
use libra::libra_types::access_path::AccessPath;
use std::collections::HashMap;
use crate::compiler::{ModuleMeta, Compiler};
use ds::MockDataSource;
use libra::move_core_types::identifier::Identifier;
use include_dir::Dir;

const STDLIB_META_ID: &str = "std_meta";
static STDLIB_DIR: Dir = include_dir!("stdlib");

pub struct Stdlib<'a> {
    pub modules: Vec<&'a str>,
}

impl Default for Stdlib<'static> {
    fn default() -> Self {
        Stdlib { modules: stdlib() }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Hash, Eq, Clone, PartialOrd, Ord)]
struct StdMeta {
    modules: Vec<ModuleId>,
}

#[derive(Debug)]
enum Module<'a> {
    Source((ModuleMeta, &'a str)),
    Binary((ModuleId, Vec<u8>)),
}

pub fn build_external_std(stdlib: Stdlib) -> Result<WriteSet, Error> {
    let ds = MockDataSource::new();
    let compiler = Compiler::new(ds.clone());

    let mut std_with_meta: HashMap<String, Module> = stdlib
        .modules
        .into_iter()
        .map(|code| {
            compiler
                .code_meta(code)
                .and_then(|meta| Ok((meta.module_name.clone(), Module::Source((meta, code)))))
        })
        .collect::<Result<HashMap<_, _>, Error>>()?;

    let mut modules = std_with_meta
        .keys()
        .map(|key| key.to_owned())
        .collect::<Vec<_>>();
    modules.sort_unstable();

    let mut ids = Vec::with_capacity(modules.len());
    for module in modules {
        build_module_with_dep(
            &module,
            &mut std_with_meta,
            &AccountAddress::default(),
            &compiler,
            &ds,
            &mut ids,
        )?;
    }

    let meta = lcs::to_bytes(&StdMeta { modules: ids })?;
    ds.publish_module_with_id(meta_module_id()?, meta)?;
    ds.to_write_set()
}

fn build_module_with_dep(
    module_name: &str,
    std_with_meta: &mut HashMap<String, Module>,
    account: &AccountAddress,
    compiler: &Compiler<MockDataSource>,
    ds: &MockDataSource,
    ids: &mut Vec<ModuleId>,
) -> Result<(), Error> {
    if let Some(module) = std_with_meta.remove(module_name) {
        match module {
            Module::Binary(binary) => {
                std_with_meta.insert(module_name.to_owned(), Module::Binary(binary));
            }
            Module::Source((meta, source)) => {
                for dep in &meta.dep_list {
                    build_module_with_dep(
                        dep.name().as_str(),
                        std_with_meta,
                        account,
                        compiler,
                        ds,
                        ids,
                    )?;
                }

                let (id, module) = build_module(&source, &account, compiler)?;
                ds.publish_module(module.clone())?;
                ids.push(id.clone());
                std_with_meta.insert(meta.module_name, Module::Binary((id, module)));
            }
        }
    }
    Ok(())
}

fn build_module(
    code: &str,
    account: &AccountAddress,
    compiler: &Compiler<MockDataSource>,
) -> Result<(ModuleId, Vec<u8>), Error> {
    compiler.compile(code, account).and_then(|module| {
        Ok(CompiledModule::deserialize(module.as_ref()).and_then(|m| Ok((m.self_id(), module)))?)
    })
}

pub fn load_std(view: &impl StateView) -> Result<Option<Vec<VerifiedModule>>, Error> {
    let module_id = meta_module_id()?;
    let meta = view.get(&AccessPath::code_access_path(&module_id))?;

    let meta: StdMeta = match meta {
        Some(meta) => lcs::from_bytes(&meta)?,
        None => return Ok(None),
    };

    let modules = meta
        .modules
        .iter()
        .map(|module_id| {
            view.get(&AccessPath::code_access_path(module_id))
                .and_then(|val| {
                    val.ok_or_else(|| {
                        Error::msg(format!("Std module [{:?}] not found.", module_id))
                    })
                })
                .and_then(|module| {
                    CompiledModule::deserialize(&module).map_err(|err| {
                        Error::msg(format!(
                            "Failed to deserialize Std module [{:?}]. Err:[{:?}]",
                            module_id, err
                        ))
                    })
                })
                .and_then(|module| {
                    VerifiedModule::new(module)
                        .map_err(|(_, status)| Error::msg(format!("{:?}", status)))
                })
        })
        .collect::<Result<_, _>>()?;

    Ok(Some(modules))
}

fn meta_module_id() -> Result<ModuleId, Error> {
    Ok(ModuleId::new(
        AccountAddress::default(),
        Identifier::new(STDLIB_META_ID)?,
    ))
}

#[derive(Serialize, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct Value {
    address: AccountAddress,
    path: String,
    value: String,
}

#[derive(Serialize)]
pub struct WS {
    write_set: Vec<Value>,
}

impl From<WriteSet> for WS {
    fn from(ws: WriteSet) -> Self {
        let write_set = ws
            .iter()
            .map(|(path, ops)| {
                let value = match ops {
                    WriteOp::Value(val) => hex::encode(val),
                    WriteOp::Deletion => "".to_owned(),
                };

                Value {
                    address: path.address,
                    path: hex::encode(&path.path),
                    value,
                }
            })
            .collect();
        WS { write_set }
    }
}

fn stdlib() -> Vec<&'static str> {
    STDLIB_DIR
        .files()
        .iter()
        .map(|f| f.contents_utf8().unwrap())
        .collect()
}

pub fn build_std() -> WriteSet {
    build_external_std(Stdlib::default()).unwrap()
}

pub fn zero_sdt() -> WriteSet {
    let ds = MockDataSource::new();
    let meta = lcs::to_bytes(&StdMeta { modules: vec![] }).unwrap();
    ds.publish_module_with_id(meta_module_id().unwrap(), meta)
        .unwrap();
    ds.to_write_set().unwrap()
}

#[cfg(test)]
pub mod tests {
    use crate::stdlib::build_std;

    #[test]
    fn test_build_std() {
        build_std();
    }
}
