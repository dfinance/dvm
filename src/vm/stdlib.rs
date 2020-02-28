use libra_types::write_set::{WriteSet, WriteOp};
use anyhow::Error;
use libra_types::account_address::AccountAddress;
use vm::CompiledModule;
use vm_runtime::data_cache::TransactionDataCache;
use crate::ds::MockDataSource;
use libra_types::language_storage::ModuleId;
use lcs;
use serde::{Deserialize, Serialize};
use libra_types::identifier::Identifier;
use bytecode_verifier::VerifiedModule;
use libra_state_view::StateView;
use libra_types::access_path::AccessPath;
use std::collections::HashMap;
use crate::vm::compiler::{Lang, ModuleMeta, Compiler};

const STDLIB_META_ID: &str = "std_meta";

pub struct Stdlib<'a> {
    pub modules: Vec<&'a str>,
    pub lang: Lang,
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

pub fn build_std_with_compiler(stdlib: Stdlib, compiler: &dyn Compiler) -> Result<WriteSet, Error> {
    let mut std_with_meta: HashMap<String, Module> = stdlib
        .modules
        .into_iter()
        .map(|code| {
            compiler
                .module_meta(&code)
                .and_then(|meta| Ok((meta.module_name.clone(), Module::Source((meta, code)))))
        })
        .collect::<Result<HashMap<_, _>, Error>>()?;

    let modules: Vec<String> = std_with_meta.keys().map(|key| key.to_owned()).collect();
    for module in modules {
        build_module_with_dep(
            &module,
            &mut std_with_meta,
            &AccountAddress::default(),
            compiler,
        )?;
    }

    let ds = MockDataSource::without_std();
    let mut data_view = TransactionDataCache::new(&ds);

    let mut ids = Vec::with_capacity(std_with_meta.len());

    for (_, module) in std_with_meta {
        match module {
            Module::Binary((id, binary)) => {
                data_view.publish_module(id.clone(), binary)?;
                ids.push(id);
            }
            Module::Source(_) => unreachable!(),
        }
    }

    ids.sort_unstable();

    let meta = lcs::to_bytes(&StdMeta { modules: ids })?;
    let std_meta_id = meta_module_id()?;
    data_view.publish_module(std_meta_id, meta)?;

    Ok(data_view.make_write_set()?)
}

pub fn build_std(stdlib: Stdlib) -> Result<WriteSet, Error> {
    let compiler = stdlib.lang.compiler();
    build_std_with_compiler(stdlib, compiler.as_ref())
}

fn build_module_with_dep(
    module_name: &str,
    std_with_meta: &mut HashMap<String, Module>,
    account: &AccountAddress,
    compiler: &dyn Compiler,
) -> Result<(), Error> {
    if let Some(module) = std_with_meta.remove(module_name) {
        match module {
            Module::Binary(binary) => {
                std_with_meta.insert(module_name.to_owned(), Module::Binary(binary));
            }
            Module::Source((meta, source)) => {
                for dep in &meta.dep_list {
                    build_module_with_dep(&dep, std_with_meta, account, compiler)?;
                }
                std_with_meta.insert(
                    meta.module_name,
                    Module::Binary(build_module(&source, &account, compiler)?),
                );
            }
        }
    }
    Ok(())
}

fn build_module(
    code: &str,
    account: &AccountAddress,
    compiler: &dyn Compiler,
) -> Result<(ModuleId, Vec<u8>), Error> {
    compiler
        .build_module(code, account, true)
        .and_then(|module| {
            Ok(CompiledModule::deserialize(&module).and_then(|m| Ok((m.self_id(), module)))?)
        })
}

pub fn load_std(view: &dyn StateView) -> Result<Option<Vec<VerifiedModule>>, Error> {
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

pub fn move_std() -> Vec<&'static str> {
    vec![
        include_str!("../../stdlib/move/address_util.move"),
        include_str!("../../stdlib/move/block.move"),
        include_str!("../../stdlib/move/bytearray_util.move"),
        include_str!("../../stdlib/move/event.move"),
        include_str!("../../stdlib/move/hash.move"),
        include_str!("../../stdlib/move/libra_account.move"),
        include_str!("../../stdlib/move/libra_coin.move"),
        include_str!("../../stdlib/move/signature.move"),
        include_str!("../../stdlib/move/transaction.move"),
        include_str!("../../stdlib/move/transaction_fee_distribution.move"),
        include_str!("../../stdlib/move/u64_util.move"),
        include_str!("../../stdlib/move/validator_config.move"),
        include_str!("../../stdlib/move/validator_set.move"),
        include_str!("../../stdlib/move/vector.move"),
    ]
}

pub fn mvir_std() -> Vec<&'static str> {
    vec![
        include_str!("../../stdlib/mvir/address_util.mvir"),
        include_str!("../../stdlib/mvir/bytearray_util.mvir"),
        include_str!("../../stdlib/mvir/gas_schedule.mvir"),
        include_str!("../../stdlib/mvir/hash.mvir"),
        include_str!("../../stdlib/mvir/wb_account.mvir"),
        include_str!("../../stdlib/mvir/wb_coins.mvir"),
        include_str!("../../stdlib/mvir/signature.mvir"),
        include_str!("../../stdlib/mvir/u64_util.mvir"),
        include_str!("../../stdlib/mvir/validator_config.mvir"),
        include_str!("../../stdlib/mvir/vector.mvir"),
        include_str!("../../stdlib/mvir/libra_time.mvir"),
        include_str!("../../stdlib/mvir/libra_transaction_timeout.mvir"),
        include_str!("../../stdlib/mvir/offer.mvir"),
        include_str!("../../stdlib/mvir/oracle.mvir"),
    ]
}
