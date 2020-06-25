use anyhow::Error;

use libra::libra_vm::access::ScriptAccess;
use libra::libra_vm::file_format::{CompiledScript, SignatureToken};

/// Procedures for verification of bytecode (restricted set of instructions and modules).
pub mod verification;

/// Get script signature.
pub fn extract_script_params(bytecode: &[u8]) -> Result<Vec<SignatureToken>, Error> {
    let script = CompiledScript::deserialize(bytecode).map_err(|err| {
        anyhow!(
            "Cannot deserialize script from provided bytecode. Error:[{}]",
            err
        )
    })?;

    let arguments = script.signature_at(script.as_inner().parameters);
    Ok(arguments.0.to_vec())
}
