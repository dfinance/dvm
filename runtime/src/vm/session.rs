use ds::DataSource;
use libra::account::{AccountAddress, CORE_CODE_ADDRESS};
use libra::ds::{RemoteCache, StructTag, TypeTag};
use libra::lcs;
use libra::module::ModuleId;
use libra::result::PartialVMError;
use libra::result::PartialVMResult;
use libra::result::StatusCode;
use libra::result::VMResult;
use libra::vm::{NativeBalance, WalletId};

/// Execution session.
#[derive(Clone)]
pub struct StateViewSession<'a, D: DataSource> {
    ds: &'a D,
    timestamp: u64,
    block: u64,
}

impl<'a, D: DataSource> StateViewSession<'a, D> {
    /// Create new execution session.
    pub fn session(
        ds: &'a D,
        timestamp: u64,
        block: u64,
    ) -> (StateViewSession<'a, D>, Box<dyn NativeBalance>) {
        (
            StateViewSession {
                ds,
                timestamp,
                block,
            },
            Box::new(Bank { ds: ds.clone() }),
        )
    }
}

struct Bank<D: DataSource> {
    ds: D,
}

impl<D: DataSource> NativeBalance for Bank<D> {
    fn get_balance(&self, wallet_id: &WalletId) -> Option<u128> {
        let balance = self.ds.get_balance(wallet_id.address, ticker(&wallet_id)?);
        match balance {
            Ok(balance) => balance,
            Err(err) => {
                warn!("Failed to get balance:'{:?}' {:?}", wallet_id, err);
                None
            }
        }
    }
}

impl<'a, D: DataSource> RemoteCache for StateViewSession<'a, D> {
    fn get_module(&self, module_id: &ModuleId) -> VMResult<Option<Vec<u8>>> {
        self.ds.get_module(module_id)
    }

    fn get_resource(
        &self,
        address: &AccountAddress,
        tag: &StructTag,
    ) -> PartialVMResult<Option<Vec<u8>>> {
        if *address == CORE_CODE_ADDRESS && tag.address == CORE_CODE_ADDRESS {
            match (tag.module.as_str(), tag.name.as_str()) {
                ("Block", "BlockMetadata") => Ok(Some(self.block.to_le_bytes().to_vec())),
                ("Time", "CurrentTimestamp") => Ok(Some(self.timestamp.to_le_bytes().to_vec())),
                ("Coins", "Price") => {
                    if tag.type_params.len() == 2 {
                        let first_part = extract_name(&tag.type_params[0])
                            .ok_or_else(|| PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR))?;
                        let second_part = extract_name(&tag.type_params[1])
                            .ok_or_else(|| PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR))?;
                        self.ds
                            .get_price(first_part, second_part)
                            .map(|price| price.map(|p| p.to_le_bytes().to_vec()))
                            .map_err(|err| {
                                PartialVMError::new(StatusCode::MISSING_DEPENDENCY)
                                    .with_message(err.to_string())
                            })
                    } else {
                        Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR))
                    }
                }
                ("Dfinance", "Info") => {
                    if tag.type_params.len() == 1 {
                        let ticker = extract_name(&tag.type_params[0])
                            .ok_or_else(|| PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR))?;
                        let info = self.ds.get_currency_info(ticker);
                        match info {
                            Ok(Some(info)) => lcs::to_bytes(&info).map(Some).map_err(|err| {
                                PartialVMError::new(StatusCode::VALUE_SERIALIZATION_ERROR)
                                    .with_message(err.to_string())
                            }),
                            Err(err) => Err(PartialVMError::new(StatusCode::MISSING_DEPENDENCY)
                                .with_message(err.to_string())),
                            Ok(None) => Ok(None),
                        }
                    } else {
                        Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR))
                    }
                }
                (_, _) => self.ds.get_resource(address, tag),
            }
        } else {
            self.ds.get_resource(address, tag)
        }
    }
}

const XFI: &str = "XFI";
const COINS: &str = "Coins";

/// Returns balance ticker.
pub fn ticker(wallet_id: &WalletId) -> Option<String> {
    if wallet_id.tag.address == CORE_CODE_ADDRESS {
        match wallet_id.tag.module.as_str() {
            XFI => Some(XFI.to_owned()),
            COINS => Some(wallet_id.tag.name.as_str().to_owned()),
            _ => None,
        }
    } else {
        None
    }
}

fn extract_name(tag: &TypeTag) -> Option<String> {
    match tag {
        TypeTag::Struct(tg) => Some(if tg.module.as_str() == XFI {
            XFI.to_owned()
        } else {
            tg.name.as_str().to_owned()
        }),
        _ => None,
    }
}
