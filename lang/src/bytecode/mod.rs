use anyhow::Error;
use libra::vm::file_format::{SignatureToken, CompiledScript};
use libra::vm::printers::TableAccess;

pub mod verification;

pub fn extract_script_params(bytecode: &[u8]) -> Result<Vec<SignatureToken>, Error> {
    let compiled_script = CompiledScript::deserialize(bytecode)
        .map_err(|err| {
            anyhow!("Cannot deserialize script from provided bytecode. Error:[{}]", err)
        })?
        .into_inner();

    let main_function = compiled_script
        .get_function_at(compiled_script.main.function)?;
    let main_function_signature = compiled_script
        .get_function_signature_at(main_function.signature)?;

    Ok(main_function_signature.arg_types.to_owned())
}
