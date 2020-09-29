#[macro_use]
extern crate include_dir;
extern crate anyhow;
extern crate libra;

use libra::prelude::*;
use anyhow::Error;
use serde::Serialize;
use std::collections::HashMap;
use ds::MockDataSource;
use include_dir::Dir;
use compiler::Compiler;

static STDLIB_DIR: Dir = include_dir!("modules");

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

pub fn zero_std() -> WriteSet {
    let ds = MockDataSource::new();
    ds.to_write_set().unwrap()
}

#[cfg(test)]
pub mod tests {
    use super::build_std;
    use test_kit::test_suite::{run_test_suite};
    use include_dir::Dir;
    use libra::logger::Logger;
    use std::collections::HashMap;

    #[test]
    fn test_build_std() {
        build_std();
    }

    static BASE_TESTS_DIR: Dir = include_dir!("tests/base");
    static STDLIB_TESTS_DIR: Dir = include_dir!("tests/stdlib");

    #[test]
    fn test_move() {
        Logger::builder().init();
        run_test_suite(dir_content(BASE_TESTS_DIR));
        run_test_suite(dir_content(STDLIB_TESTS_DIR));
    }

    fn dir_content(dir: Dir) -> HashMap<String, String> {
        dir.files()
            .iter()
            .map(|f| {
                (
                    f.path().file_name().unwrap().to_str().unwrap().to_owned(),
                    f.contents_utf8().unwrap().to_owned(),
                )
            })
            .collect()
    }
}
