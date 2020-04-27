use anyhow::Error;
use libra::libra_vm::file_format::{SignatureToken, CompiledScript};
use libra::libra_vm::access::ScriptAccess;

pub mod disassembler;
pub mod verification;

pub fn extract_script_params(bytecode: &[u8]) -> Result<Vec<SignatureToken>, Error> {
    let compiled_script = CompiledScript::deserialize(bytecode).map_err(|err| {
        anyhow!(
            "Cannot deserialize script from provided bytecode. Error:[{}]",
            err
        )
    })?;

    let main_function =
        compiled_script.function_handle_at(compiled_script.as_inner().main.function);
    let signature = compiled_script.signature_at(main_function.parameters);
    Ok(signature.0.to_vec())
}
