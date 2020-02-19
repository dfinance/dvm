use libra_types::language_storage::TypeTag;
use std::collections::VecDeque;
use vm::gas_schedule::{CostTable, NativeCostIndex};
use vm_runtime_types::{pop_arg, native_functions::dispatch::NativeResult};
use vm_runtime_types::values::Value;
use libra_types::vm_error::VMStatus;
use libra_types::byte_array::ByteArray;
use vm_runtime_types::native_functions::dispatch::native_gas;
use crate::vm::native::Function;
use crate::{module};

#[derive(Debug)]
pub struct PrintByteArray {}

impl Function for PrintByteArray {
    fn call(
        &self,
        _ty_args: Vec<TypeTag>,
        mut arguments: VecDeque<Value>,
        cost_table: &CostTable,
    ) -> Result<NativeResult, VMStatus> {
        let cost = native_gas(cost_table, NativeCostIndex::LENGTH, 1);
        let print_arg: ByteArray = pop_arg!(arguments, ByteArray);
        println!(
            "native fn PrintByteArray called with: {}",
            hex::encode(print_arg.as_bytes())
        );
        Ok(NativeResult::ok(cost, vec![]))
    }
}

module! {
    Dbg;
    [PrintByteArray::<All>print_byte_array fn (ByteArray)->()]
}
