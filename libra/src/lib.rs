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
pub extern crate compiler;
pub extern crate libra_logger;
pub extern crate ir_to_bytecode;
pub extern crate move_vm_runtime;
pub extern crate move_vm_natives;

pub mod prelude {
    pub use crate::account::*;
    pub use crate::vm_result::*;
    pub use crate::ds::*;
    pub use crate::module::*;
}

pub mod module {
    pub use move_core_types::language_storage::ModuleId;
    pub use libra_types::transaction::Module;
    pub use libra_vm::CompiledModule;
}

pub mod account {
    pub use libra_types::account_address::AccountAddress;
    pub use libra_types::account_config::CORE_CODE_ADDRESS;
}

pub mod vm_result {
    pub use libra_types::vm_status::{StatusCode, VMStatus};
    pub use libra_vm::errors::VMResult;
}

pub mod ds {
    pub use libra_state_view::StateView;
    pub use libra_types::access_path::AccessPath;
    pub use move_vm_runtime::data_cache::RemoteCache;
    pub use libra_types::write_set::{WriteOp, WriteSet, WriteSetMut};
}