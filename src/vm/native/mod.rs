pub mod dbg;

use std::collections::VecDeque;
use vm_runtime_types::native_functions::dispatch::NativeResult;
use libra_types::language_storage::TypeTag;
use vm::gas_schedule::CostTable;
use vm::errors::VMResult;
use vm_runtime_types::values::Value;
use anyhow::Error;

pub fn init_native() -> Result<(), Error> {
    dbg::PrintByteArray {}.reg_function();

    Ok(())
}

pub trait Function {
    fn call(
        &self,
        ty_args: Vec<TypeTag>,
        arguments: VecDeque<Value>,
        cost_table: &CostTable,
    ) -> VMResult<NativeResult>;
}

pub trait Dispatch {
    fn dispatch(
        ty_args: Vec<TypeTag>,
        arguments: VecDeque<Value>,
        cost_table: &CostTable,
    ) -> VMResult<NativeResult>;
}

pub trait Reg {
    fn reg_function(self);
}

#[macro_export]
macro_rules! module {
    ($module: ident; [$($function_type:ident::<$($kinds:ident),*>$name:ident fn ($($args:expr),*) -> ($($ret:expr),*)),*]) => {
        #[allow(non_snake_case)]
        pub mod $module {
            $(
                pub mod $name {
                    use std::collections::{VecDeque, HashMap};
                    use libra_types::account_config;
                    use once_cell::sync::OnceCell;
                    use vm::gas_schedule::CostTable;
                    use libra_types::language_storage::{ModuleId, TypeTag};
                    use libra_types::identifier::Identifier;
                    use vm_runtime_types::values::Value;
                    use crate::vm::native::{Dispatch, Reg, Function};
                    use vm::errors::VMResult;
                    use vm_runtime_types::native_functions::dispatch::{NativeFunction, NativeResult, EXTERNAL_NATIVE_FUNCTIONS};
                    use vm::file_format::FunctionSignature;
                    use super::super::$function_type as FType;
                    use std::sync::Mutex;
                    use vm::file_format::{SignatureToken::*, Kind::*};

                    static INSTANCE : OnceCell<FType> = OnceCell::new();

                    struct $function_type;

                    impl Reg for FType {
                        fn reg_function(self) {
                            INSTANCE.set(self).unwrap();

                            let expected_signature = FunctionSignature {
                                return_types: vec![$($ret),*],
                                arg_types: vec![$($args),*],
                                type_formals: vec![$($kinds),*],
                            };

                            let func = NativeFunction {
                                dispatch: $function_type::dispatch,
                                expected_signature,
                            };

                            let ext_funcs = &mut EXTERNAL_NATIVE_FUNCTIONS.get_or_init(|| Mutex::new(Some(HashMap::new()))).lock().unwrap();

                            match ext_funcs.as_mut() {
                                Some(ext_funcs) => {
                                    ext_funcs.entry(ModuleId::new(account_config::core_code_address(), Identifier::new(stringify!($module)).unwrap()))
                                    .or_insert_with(HashMap::new)
                                    .insert(Identifier::new(stringify!($name)).unwrap(), func);
                                }
                                None => { panic!("Runtime is already registered."); }
                            }
                        }
                    }

                    impl Dispatch for $function_type {
                        fn dispatch(ty_args: Vec<TypeTag>, arguments: VecDeque<Value>, cost_table: &CostTable) -> VMResult<NativeResult> {
                            INSTANCE.get().expect("Expected instance").call(ty_args, arguments, cost_table)
                        }
                    }
                }
            ),*
        }
    }
}
