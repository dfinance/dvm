use libra_types::language_storage::TypeTag;
use std::collections::VecDeque;
use vm::gas_schedule::{CostTable, NativeCostIndex};
use vm_runtime_types::{pop_arg, native_functions::dispatch::NativeResult};
use vm_runtime_types::values::Value;
use libra_types::vm_error::VMStatus;
use libra_types::byte_array::ByteArray;
use vm_runtime_types::native_functions::dispatch::native_gas;
use crate::vm::native::Function;
use crate::module;
use std::sync::{
    Arc,
    Mutex,
};

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
        println!("{}", hex::encode(print_arg.as_bytes()));
        Ok(NativeResult::ok(cost, vec![]))
    }
}

#[derive(Clone, Debug)]
pub struct DumpU64 {
    value: Arc<Mutex<Option<u64>>>
}

impl DumpU64 {
    pub fn new() -> DumpU64 {
        DumpU64 {
            value: Arc::new(Mutex::new(None))
        }
    }

    pub fn get(&self) -> Option<u64> {
        self.value.lock().unwrap().clone()
    }

    pub fn store(&self, val: Option<u64>) {
        *self.value.lock().unwrap() = val;
    }
}

impl Function for DumpU64 {
    fn call(
        &self,
        _ty_args: Vec<TypeTag>,
        mut arguments: VecDeque<Value>,
        cost_table: &CostTable,
    ) -> Result<NativeResult, VMStatus> {
        let cost = native_gas(cost_table, NativeCostIndex::LENGTH, 1);
        self.store(Some(pop_arg!(arguments, u64)));
        Ok(NativeResult::ok(cost, vec![]))
    }
}

module! {
    Dbg;
    [
        PrintByteArray::<All>print_byte_array fn (ByteArray)->();
        DumpU64::<All>dump_u64 fn (U64)->()
    ]
}
