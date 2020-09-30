use libra::{prelude::*, vm::StructTag};
use serde::{Deserialize, Serialize};

const COIN_MODULE: &str = "Coins";
const PRICE_STRUCT: &str = "Price";

const XFI_MODULE: &str = "XFI";
const XFI_RESOURCE: &str = "T";

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
    pub price: u128,
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
    if curr == XFI_MODULE {
        TypeTag::Struct(StructTag {
            address: CORE_CODE_ADDRESS,
            name: Identifier::new(XFI_RESOURCE).expect("Valid module name."),
            module: Identifier::new(XFI_MODULE).expect("Valid currency name."),
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

/// Deserializable `u64` for lcs.
#[derive(Deserialize, Debug, PartialEq, Eq)]
pub struct U64Store {
    /// Internal value.
    pub val: u64,
}

/// Deserializable `AccountAddress` for lcs.
#[derive(Deserialize, Debug, PartialEq, Eq)]
pub struct AddressStore {
    /// Internal value.
    pub val: AccountAddress,
}

/// Deserializable `Vec<u8>` for lcs.
#[derive(Deserialize, Debug, PartialEq, Eq)]
pub struct VectorU8Store {
    /// Internal value.
    pub val: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use crate::resources::*;
    use libra::prelude::CORE_CODE_ADDRESS;

    const ACCOUNT_MODULE: &str = "Account";
    const BALANCE_STRUCT: &str = "Balance";

    const DFINANCE_MODULE: &str = "Dfinance";
    const INFO_STRUCT: &str = "Info";

    #[test]
    pub fn test_oracle_metadata() {
        let vector = oracle_metadata("XFI", "BTC").access_vector();
        assert_eq!(
            vector,
            hex::decode("018c2f213d25358a39f9370a494dbe4bd80f84734137a01ec8f468c3b2ef16360a")
                .unwrap()
        );

        let vector = oracle_metadata("BTC", "ETH").access_vector();
        assert_eq!(
            vector,
            hex::decode("01a7183ec0c4d32fd9a2705e1e6844035c5238598bf45167742e9db3735af96cc1")
                .unwrap()
        );
    }

    #[test]
    pub fn test_balance_vector() {
        fn balance_vector(curr: &str) -> StructTag {
            StructTag {
                address: CORE_CODE_ADDRESS,
                name: Identifier::new(BALANCE_STRUCT).expect("Valid struct name."),
                module: Identifier::new(ACCOUNT_MODULE).expect("Valid module name."),
                type_params: vec![currency_type(curr)],
            }
        }

        let vector = balance_vector("eth").access_vector();
        assert_eq!(
            vector,
            hex::decode("0138f4f2895881c804de0e57ced1d44f02e976f9c6561c889f7b7eef8e660d2c9a")
                .unwrap()
        );

        let vector = balance_vector("xfi").access_vector();
        assert_eq!(
            vector,
            hex::decode("01226844e85ad6e3867f4ff1a4300e71ed6057538631a5a5330512772b7104b585")
                .unwrap()
        );
    }

    #[test]
    pub fn test_currency_info_vector() {
        fn currency_info_vector(curr: &str) -> StructTag {
            StructTag {
                address: CORE_CODE_ADDRESS,
                name: Identifier::new(INFO_STRUCT).expect("Valid struct name."),
                module: Identifier::new(DFINANCE_MODULE).expect("Valid module name."),
                type_params: vec![currency_type(curr)],
            }
        }

        let vector = currency_info_vector("eth").access_vector();
        assert_eq!(
            vector,
            hex::decode("012a00668b5325f832c28a24eb83dffa8295170c80345fbfbf99a5263f962c76f4")
                .unwrap()
        );

        let vector = currency_info_vector("xfi").access_vector();
        assert_eq!(
            vector,
            hex::decode("01b9ed21c23abf8c7a53fb868a36e106d45394c30127fb722f8dd2d45aae719585")
                .unwrap()
        );
    }

    #[test]
    pub fn test_time_metadata() {
        let vector = time_metadata().access_vector();
        assert_eq!(
            vector,
            hex::decode("01bb4980dfba817aaa64cb4b3a75ee1e1d8352111dead64c5c4f05fb7b4c85bb3e")
                .unwrap()
        );
    }

    #[test]
    pub fn test_block_metadata() {
        let vector = block_metadata().access_vector();
        assert_eq!(
            vector,
            hex::decode("01ada6f79e8eddfdf986687174de1000df3c5fa45e9965ece812fed33332ec543a")
                .unwrap()
        );
    }
}
