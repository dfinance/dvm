use libra::{prelude::*, vm::*};
use serde_derive::{Deserialize, Serialize};

/// Height of the current block.
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct BlockMetadata {
    /// Block height.
    pub height: u64,
}

/// A singleton resource holding the current Unix time in seconds.
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct CurrentTimestamp {
    /// Unix time stamp in seconds
    pub seconds: u64,
}

/// Currency price.
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct Price {
    /// Currency price.
    pub price: u64,
}

/// Returns oracle metadata struct tag.
pub fn oracle_metadata(first: String, second: String) -> StructTag {
    StructTag {
        address: CORE_CODE_ADDRESS,
        name: Identifier::new("Price").expect("Valid module name."),
        module: Identifier::new("Currency").expect("Valid module name."),
        type_params: vec![
            TypeTag::Struct(StructTag {
                address: CORE_CODE_ADDRESS,
                name: Identifier::new("Currency").expect("Valid module name."),
                module: Identifier::new(first).expect("Valid currency name."),
                type_params: vec![],
            }),
            TypeTag::Struct(StructTag {
                address: CORE_CODE_ADDRESS,
                name: Identifier::new("Currency").expect("Valid module name."),
                module: Identifier::new(second).expect("Valid currency name."),
                type_params: vec![],
            }),
        ],
    }
}

/// Returns block metadata struct tag.
pub fn block_metadata() -> StructTag {
    StructTag {
        address: CORE_CODE_ADDRESS,
        name: Identifier::new("BlockMetadata").expect("Valid module name."),
        module: Identifier::new("Block").expect("Valid module name."),
        type_params: vec![],
    }
}

/// Returns time metadata struct tag.
pub fn time_metadata() -> StructTag {
    StructTag {
        address: CORE_CODE_ADDRESS,
        name: Identifier::new("CurrentTimestamp").expect("Valid module name."),
        module: Identifier::new("Time").expect("Valid module name."),
        type_params: vec![],
    }
}
