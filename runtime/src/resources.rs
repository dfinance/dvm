use libra::{prelude::*, vm::*};
use serde_derive::{Deserialize, Serialize};

const COIN_MODULE: &str = "Coins";
const PRICE_STRUCT: &str = "Price";

const DFI_MODULE: &str = "DFI";
const DFI_RESOURCE: &str = "T";

const BLOCK_RESOURCE: &str = "BlockMetadata";

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
pub fn oracle_metadata(first: &str, second: &str) -> StructTag {
    StructTag {
        address: CORE_CODE_ADDRESS,
        name: Identifier::new(PRICE_STRUCT).expect("Valid struct name."),
        module: Identifier::new(COIN_MODULE).expect("Valid module name."),
        type_params: vec![currency_type(first), currency_type(second)],
    }
}

fn currency_type(curr: &str) -> TypeTag {
    let curr = curr.to_uppercase();
    if curr == DFI_MODULE {
        TypeTag::Struct(StructTag {
            address: CORE_CODE_ADDRESS,
            name: Identifier::new(DFI_RESOURCE).expect("Valid module name."),
            module: Identifier::new(DFI_MODULE).expect("Valid currency name."),
            type_params: vec![],
        })
    } else {
        TypeTag::Struct(StructTag {
            address: CORE_CODE_ADDRESS,
            name: Identifier::new(curr).expect("Valid module name."),
            module: Identifier::new(COIN_MODULE).expect("Valid currency name."),
            type_params: vec![],
        })
    }
}

/// Returns block metadata struct tag.
pub fn block_metadata() -> StructTag {
    StructTag {
        address: CORE_CODE_ADDRESS,
        name: Identifier::new(BLOCK_RESOURCE).expect("Valid module name."),
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
