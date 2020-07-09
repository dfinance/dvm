pub extern crate move_lang;
pub extern crate libra_crypto;
pub extern crate language_e2e_tests;
pub extern crate libra_vm;
pub extern crate move_core_types;
pub extern crate bytecode_source_map;
pub extern crate move_vm_types;
pub extern crate libra_state_view;
pub extern crate libra_config;
pub extern crate libra_types;
pub extern crate move_ir_types;
pub extern crate stdlib;
pub extern crate bytecode_verifier;
pub extern crate lcs;
extern crate compiler as libra_compiler;
pub extern crate libra_logger;
pub extern crate ir_to_bytecode;
pub extern crate move_vm_runtime;
pub extern crate move_vm_natives;

pub mod prelude {
    pub use crate::account::*;
    pub use crate::result::*;
    pub use crate::ds::*;
    pub use crate::module::*;
}

pub mod module {
    pub use move_core_types::language_storage::ModuleId;
    pub use libra_types::transaction::Module;
    pub use libra_vm::access::{ModuleAccess, ScriptAccess};
    pub use libra_vm::file_format::{Bytecode, CompiledScript, CompiledModule, ModuleHandle, SignatureToken};
    pub use move_lang::compiled_unit::CompiledUnit;
    pub use move_lang::parser::ast::{Definition, ModuleDefinition, Script};
}

pub mod account {
    pub use libra_types::account_address::AccountAddress;
    pub use libra_types::account_config::CORE_CODE_ADDRESS;
    pub use move_core_types::identifier::Identifier;
}

pub mod result {
    pub use libra_types::vm_status::{StatusCode, VMStatus};
    pub use libra_vm::errors::VMResult;
}

pub mod ds {
    pub use libra_state_view::StateView;
    pub use libra_types::access_path::AccessPath;
    pub use move_vm_runtime::data_cache::RemoteCache;
    pub use libra_types::write_set::{WriteOp, WriteSet, WriteSetMut};
}

pub mod compiler {
    pub use move_lang::{compiled_unit, errors, parse_program, compile_program};
    pub use move_lang::parser::ast::*;
    pub use move_lang::shared::Address;
    pub use move_lang::errors::{FilesSourceText, Errors, output_errors};
    pub use move_lang::name_pool::ConstPool;
    pub use move_lang::move_check;
}

pub mod file_format {
    pub use libra_vm::file_format::*;
}