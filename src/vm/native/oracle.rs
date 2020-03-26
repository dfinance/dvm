use std::fmt;
use std::collections::VecDeque;
use libra::{vm, libra_types, libra_state_view, vm_runtime_types, libra_crypto};
use libra_types::language_storage::TypeTag;
use vm::gas_schedule::{CostTable, GasCost};
use vm_runtime_types::{pop_arg, native_functions::dispatch::NativeResult};
use vm_runtime_types::values::Value;
use libra_types::vm_error::{VMStatus, StatusCode};
use libra_state_view::StateView;
use libra_types::access_path::AccessPath;
use libra_types::account_config::core_code_address;
use libra_crypto::hash::{DefaultHasher, CryptoHasher};
use byteorder::{ByteOrder, LittleEndian};
use crate::vm::native::Function;
use crate::module;

const COST: u64 = 929;
const PRICE_ORACLE_TAG: u8 = 255;

pub type View = dyn StateView + Sync + Send;

pub struct PriceOracle {
    view: Box<View>,
}

impl PriceOracle {
    pub fn new(view: Box<View>) -> PriceOracle {
        PriceOracle { view }
    }

    pub fn make_path(ticker_pair: u64) -> Result<AccessPath, VMStatus> {
        let mut hasher = DefaultHasher::default();
        let mut buf = [0; 8];
        LittleEndian::write_u64(&mut buf, ticker_pair);
        hasher.write(&buf);
        let mut hash = hasher.finish().to_vec();
        hash.insert(0, PRICE_ORACLE_TAG);

        Ok(AccessPath::new(core_code_address(), hash))
    }
}

impl fmt::Debug for PriceOracle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "PriceOracle")
    }
}

impl Function for PriceOracle {
    fn call(
        &self,
        _ty_args: Vec<TypeTag>,
        mut arguments: VecDeque<Value>,
        _cost_table: &CostTable,
    ) -> Result<NativeResult, VMStatus> {
        let result = Self::make_path(pop_arg!(arguments, u64))
            .and_then(|path| {
                self.view.get(&path).map_err(|err| {
                    VMStatus::new(StatusCode::STORAGE_ERROR)
                        .with_sub_status(1)
                        .with_message(err.to_string())
                })
            })
            .and_then(|price| match price {
                Some(price) => {
                    if price.len() != 8 {
                        Err(VMStatus::new(StatusCode::TYPE_MISMATCH)
                            .with_sub_status(2)
                            .with_message("Invalid prise size".to_owned()))
                    } else {
                        Ok(LittleEndian::read_u64(&price))
                    }
                }
                None => Err(VMStatus::new(StatusCode::STORAGE_ERROR)
                    .with_sub_status(2)
                    .with_message("Price is not found".to_owned())),
            });

        let cost = GasCost::new(COST, 1);
        match result {
            Ok(price) => Ok(NativeResult::ok(cost.total(), vec![Value::u64(price)])),
            Err(status) => Ok(NativeResult::err(cost.total(), status)),
        }
    }
}

module! {
    Oracle;
    [PriceOracle::<All>get_price fn (U64)->(U64)]
}
