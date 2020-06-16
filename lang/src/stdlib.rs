use libra::libra_types::write_set::{WriteSet, WriteOp};
use anyhow::Error;
use libra::libra_types::account_address::AccountAddress;
use serde::Serialize;
use std::collections::HashMap;
use ds::MockDataSource;
use include_dir::Dir;
use compiler::Compiler;
use libra::move_core_types::language_storage::CORE_CODE_ADDRESS;

static STDLIB_DIR: Dir = include_dir!("stdlib");

#[derive(Debug, Clone)]
pub struct Stdlib {
    pub modules: HashMap<String, String>,
}

impl Default for Stdlib {
    fn default() -> Self {
        Stdlib { modules: stdlib() }
    }
}

pub fn build_external_std(stdlib: Stdlib) -> Result<WriteSet, Error> {
    let ds = MockDataSource::new();
    let compiler = Compiler::new(ds.clone());
    let modules = compiler.compile_source_map(stdlib.modules, Some(CORE_CODE_ADDRESS))?;

    for module in modules {
        ds.publish_module(module.1)?;
    }

    ds.to_write_set()
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

fn stdlib() -> HashMap<String, String> {
    STDLIB_DIR
        .files()
        .iter()
        .map(|f| {
            (
                f.path().file_name().unwrap().to_str().unwrap().to_owned(),
                f.contents_utf8().unwrap().to_owned(),
            )
        })
        .collect()
}

pub fn build_std() -> WriteSet {
    build_external_std(Stdlib::default()).unwrap()
}

pub fn zero_sdt() -> WriteSet {
    let ds = MockDataSource::new();
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
