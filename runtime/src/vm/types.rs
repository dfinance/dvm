use anyhow::*;
use std::fmt;
use libra::{prelude::*, vm::*, gas::*};
use std::collections::HashMap;

/// Result enum for ExecutionResult
pub type VmResult = Result<ExecutionResult, VMStatus>;

const GAS_AMOUNT_MAX_VALUE: u64 = u64::MAX / 1000;

/// Stores gas metadata for vm execution.
#[derive(Debug)]
pub struct Gas {
    /// Max gas units to be used in transaction execution.
    max_gas_amount: u64,
    /// Price in `XFI` coins per unit of gas.
    gas_unit_price: u64,
}

impl Gas {
    /// Constructor.
    pub fn new(max_gas_amount: u64, gas_unit_price: u64) -> Result<Gas> {
        ensure!(
            max_gas_amount < GAS_AMOUNT_MAX_VALUE,
            "max_gas_amount value must be in the range from 0 to {}",
            GAS_AMOUNT_MAX_VALUE
        );

        Ok(Gas {
            max_gas_amount,
            gas_unit_price,
        })
    }

    /// Returns max gas units to be used in transaction execution.
    pub fn max_gas_amount(&self) -> u64 {
        self.max_gas_amount
    }

    /// Returns price in `DFI` coins per unit of gas.
    pub fn gas_unit_price(&self) -> u64 {
        self.gas_unit_price
    }
}

/// Result of transaction execution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionResult {
    /// Changes to the chain.
    pub write_set: WriteSet,
    /// Emitted events.
    pub events: Vec<ContractEvent>,
    /// Native balance operation.
    pub wallet_ops: HashMap<WalletId, BalanceOperation>,
    /// Number of gas units used for execution.
    pub gas_used: u64,
    /// Status of execution.
    pub status: VMError,
}

impl ExecutionResult {
    /// Creates `ExecutionResult` out of resulting chain data cache and `vm_result`.
    pub fn new(
        cost_strategy: CostStrategy,
        gas_meta: Gas,
        vm_result: VMResult<TransactionEffects>,
    ) -> ExecutionResult {
        let gas_used = GasUnits::new(gas_meta.max_gas_amount)
            .sub(cost_strategy.remaining_gas())
            .get();

        vm_result
            .and_then(|effects| {
                txn_effects_to_writeset_and_events_cached(&mut (), effects).map_err(|err| {
                    PartialVMError::new(err.status_code()).finish(Location::Undefined)
                })
            })
            .map(|(write_set, events, wallet_ops)| ExecutionResult {
                write_set,
                events,
                wallet_ops,
                gas_used,
                status: PartialVMError::new(StatusCode::EXECUTED).finish(Location::Undefined),
            })
            .unwrap_or_else(|status| ExecutionResult {
                write_set: WriteSetMut::default().freeze().expect("Impossible error."),
                events: vec![],
                wallet_ops: Default::default(),
                gas_used,
                status,
            })
    }
}

/// Module transaction.
#[derive(Clone)]
pub struct ModuleTx {
    code: Vec<u8>,
    sender: AccountAddress,
}

impl ModuleTx {
    /// Constructor.
    pub fn new(code: Vec<u8>, sender: AccountAddress) -> ModuleTx {
        ModuleTx { code, sender }
    }

    /// Returns module bytecode.
    pub fn code(&self) -> &[u8] {
        &self.code
    }

    /// Convert into internal data.
    pub fn into_inner(self) -> (Vec<u8>, AccountAddress) {
        (self.code, self.sender)
    }
}

impl fmt::Debug for ModuleTx {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Module")
            .field("code", &hex::encode(&self.code))
            .field("sender", &self.sender)
            .finish()
    }
}

/// Script bytecode + passed arguments and type parameters.
pub struct ScriptTx {
    code: Vec<u8>,
    args: Vec<Value>,
    type_args: Vec<TypeTag>,
    senders: Vec<AccountAddress>,
    timestamp: u64,
    block: u64,
}

/// Script transaction.
impl ScriptTx {
    /// Constructor.
    pub fn new(
        code: Vec<u8>,
        args: Vec<Value>,
        type_args: Vec<TypeTag>,
        senders: Vec<AccountAddress>,
        timestamp: u64,
        block: u64,
    ) -> Result<Self> {
        ensure!(
            !senders.is_empty(),
            "senders value must be in the range from 0 to ",
        );
        Ok(ScriptTx {
            code,
            args,
            type_args,
            senders,
            timestamp,
            block,
        })
    }

    /// Script bytecode.
    pub fn code(&self) -> &[u8] {
        &self.code
    }

    /// Parameters passed to main() function.
    pub fn args(&self) -> &[Value] {
        &self.args
    }

    /// Convert into internal data.
    pub fn into_inner(
        self,
    ) -> (
        Vec<u8>,
        Vec<Value>,
        Vec<TypeTag>,
        Vec<AccountAddress>,
        u64,
        u64,
    ) {
        (
            self.code,
            self.args,
            self.type_args,
            self.senders,
            self.timestamp,
            self.block,
        )
    }
}

impl fmt::Debug for ScriptTx {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Script")
            .field("code", &hex::encode(&self.code))
            .field("args", &self.args)
            .field("type_args", &self.type_args)
            .field("senders", &self.senders)
            .finish()
    }
}
